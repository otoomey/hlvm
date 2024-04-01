use std::borrow::Cow;

use super::{reqrsp::{ReqPort, RspPort}, Node, Port};

#[derive(Clone)]
pub struct Src<T: Clone + 'static> {
    buffer: Vec<T>,
    index: usize,
    pub req: ReqPort<Option<T>>
}

impl<T: Clone + 'static> Src<T> {
    pub fn new(buffer: Vec<T>) -> Self {
        Self {
            buffer,
            index: 0,
            req: ReqPort::new(None)
        }
    }
}

impl<T: Clone + 'static> Node for Src<T> {
    fn get_port_ids(&self) -> Cow<[super::Port]> {
        Cow::from(vec![self.req.port])
    }

    fn get_port(&self, _: usize) -> &dyn std::any::Any {
        &self.req
    }

    fn csim<'a>(&mut self, _: &super::Ctx<'a, Self>) -> bool {
        self.req.data = self.buffer.get(self.index)
            .map(|x| x.clone());
        let valid = self.index < self.buffer.len();
        if valid != self.req.valid {
            self.req.valid = valid;
            true
        } else {
            self.req.valid = valid;
            false
        }
    }

    fn edge<'a>(&mut self, ctx: &super::Ctx<'a, Self>) {
        if let Some(port) = ctx.port::<RspPort<Option<T>>>(self.req.port) {
            if self.req.valid && port.ready {
                self.index += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::stdcells::{sink::DebugSink, Sim};

    use super::Src;

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    struct Msg(i32);

    #[test]
    fn it_works() {
        let msgs = vec![
            Msg(0),
            Msg(1),
            Msg(2),
            Msg(3),
        ];
        let mut sim = Sim::new(37);

        let sink = DebugSink::<Msg>::new("sink".to_string());
        let src = Src::<Msg>::new(msgs.clone());
        let sink_id = sim.add_node(sink);
        let src_id = sim.add_node(src);

        let sink = sim.get_node_mut::<DebugSink<Msg>>(&sink_id).unwrap();
        sink.rsp.port = src_id.port(0);

        let src = sim.get_node_mut::<Src<Msg>>(&src_id).unwrap();
        src.req.port = sink_id.port(0);

        sim.sim_cycle();
        sim.sim_cycle();
        sim.sim_cycle();
        sim.sim_cycle();

        // ensure that the received messages matched the ones we sent
        let sink = sim.get_node_mut::<DebugSink<Msg>>(&sink_id).unwrap();
        let rec: Vec<Msg> = sink.rec.iter().map(|m| m.unwrap()).collect();
        assert_eq!(msgs, rec);
    }
}
