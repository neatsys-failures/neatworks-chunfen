use crate::actor::State;

pub trait PureState<'input> {
    type Input;
    type Output<'output>
    where
        Self: 'output;

    fn update(&mut self, input: Self::Input) -> Self::Output<'_>;

    fn install<S>(self, state: S) -> Install<Self, S>
    where
        Self: Sized,
    {
        Install(self, state)
    }
}

pub trait App {
    fn update(&mut self, op_num: u32, op: &[u8]) -> Vec<u8>;
}

impl<'i, A: App> PureState<'i> for A {
    type Input = (u32, &'i [u8]);
    type Output<'o> = Vec<u8> where A: 'o;

    fn update(&mut self, input: Self::Input) -> Self::Output<'_> {
        let (op_num, op) = input;
        self.update(op_num, op)
    }
}

// the name is too abstract...
pub struct Install<A, S>(A, S);

impl<'i, A, S> State<'i> for Install<A, S>
where
    A: PureState<'i>,
    S: for<'o> State<'o, Message = A::Output<'o>>,
{
    type Message = A::Input;

    fn update(&mut self, message: Self::Message) {
        self.1.update(self.0.update(message))
    }
}
