use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
};

use murmesh_bincode::{de, ser};
use murmesh_core::{
    actor::{Drive, State, Wire},
    app::{Closure, FunctionalState},
    transport, Dispatch,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::spawn;

/// if `M` is `Ord`, then a sorted `Message<M>` will have deterministic content.
pub type Message<M> = Vec<(M, IpAddr)>;

pub struct Service<M, E, F> {
    egress: E,
    finished: F,

    accumulated: HashMap<SocketAddr, M>,
    count: usize,
}

impl<M, E, F> Service<M, E, F> {
    pub fn new(egress: E, finished: F, count: usize) -> Self {
        Self {
            egress,
            finished,
            accumulated: Default::default(),
            count,
        }
    }
}

impl<M, E, F> State<'_> for Service<M, E, F>
where
    E: for<'m> State<'m, Message = (SocketAddr, Message<M>)>,
    F: for<'m> State<'m, Message = ()>,
    M: Clone,
{
    type Message = (SocketAddr, M);

    fn update(&mut self, message: Self::Message) {
        assert!(self.accumulated.len() < self.count);
        let (remote, message) = message;
        let prev = self.accumulated.insert(remote, message);
        assert!(prev.is_none());

        if self.accumulated.len() == self.count {
            let message = Vec::from_iter(
                self.accumulated
                    .iter()
                    .map(|(addr, message)| (message.clone(), addr.ip())),
            );
            for &remote in self.accumulated.keys() {
                self.egress.update((remote, message.clone()))
            }
            self.finished.update(())
        }
    }
}

pub async fn use_barrier<M>(addr: SocketAddr, service: SocketAddr, payload: M) -> Message<M>
where
    M: Serialize + DeserializeOwned + Send + 'static,
{
    let message = Wire::default();
    let mut connection = murmesh_tcp::Connection::connect(
        addr,
        service,
        transport::Lift(de())
            .install(Closure::from(|(_, message)| message).install(message.state())),
        Wire::default().state(),
    )
    .await;
    let mut dispatch = Dispatch::default();
    dispatch.insert_state(connection.remote_addr, connection.out_state());
    let connection = spawn(async move { connection.start().await });

    transport::Lift(ser())
        .install(Closure::from(From::from).install(dispatch))
        .update((service, payload));

    let message = Drive::from(message).recv().await.unwrap();
    connection.abort();
    message
}

pub async fn provide_barrier<M>(addr: SocketAddr, count: usize)
where
    M: Clone + Serialize + DeserializeOwned + Send + 'static,
{
    let app_wire = Wire::default();
    let finished = Wire::default();
    let disconnected = Wire::default();

    let listener = murmesh_tcp::Listener::bind(addr);
    let mut dispatch = Dispatch::default();
    let mut connections = Vec::new();
    for _ in 0..count {
        let mut connection = listener
            .accept(
                transport::Lift(de::<M>()).install(app_wire.state()),
                disconnected.state(),
            )
            .await;
        dispatch.insert_state(connection.remote_addr, connection.out_state());
        connections.push(spawn(async move { connection.start().await }));
    }
    let app = Service::new(
        transport::Lift(ser()).install(Closure::from(From::from).install(dispatch)),
        finished.state(),
        count,
    );

    let app = spawn(async move { Drive::from(app_wire).run(app).await });
    Drive::from(finished).recv().await.unwrap();
    for connection in connections {
        connection.await.unwrap()
    }
    app.await.unwrap()
}
