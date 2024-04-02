use std::{borrow::Cow, collections::VecDeque};

use super::{reqrsp::{ReqPort, RspPort}, Node};

#[derive(Clone)]
pub struct Fifo<T: Clone + Eq + PartialEq + Default + 'static> {
    buffer: VecDeque<T>,
    capacity: usize,
    zero_cycle: bool,
    pub rsp: RspPort<T>,
    pub req: ReqPort<T>,
}

impl<T: Clone + 'static + Eq + PartialEq + Default> Fifo<T> {
    pub fn new(capacity: usize, zero_cycle: bool) -> Self {
        let buffer = VecDeque::with_capacity(capacity);
        Self {
            buffer,
            capacity,
            zero_cycle,
            req: ReqPort::new(Default::default()),
            rsp: RspPort::new()
        }
    }
}

impl<T: Clone + 'static + Eq + PartialEq + Default> Node for Fifo<T> {
    fn get_port_ids(&self) -> std::borrow::Cow<[super::Port]> {
        Cow::from(vec![self.rsp.port, self.req.port])
    }

    fn get_port(&self, port: usize) -> &dyn std::any::Any {
        match port {
            0 => &self.rsp,
            1 => &self.req,
            _ => { panic!("Port {} is out of range for fifo; expected 0 or 1.", port) }
        }
    }

    fn csim<'a>(&mut self, ctx: &super::Ctx<'a, Self>) -> bool {
        self.rsp.ready = self.buffer.len() < self.capacity;
        
        let ivalid = ctx.port::<ReqPort<T>>(self.rsp.port)
            .map(|req| req.valid)
            .unwrap_or(false);

        let idata = ctx.port::<ReqPort<T>>(self.rsp.port)
            .map(|req| req.data.clone())
            .unwrap_or_default();

        self.req.valid = self.buffer.len() > 0 || (ivalid && self.zero_cycle);

        self.req.data = match self.zero_cycle && self.buffer.is_empty() {
            true => idata,
            false => self.buffer.front()
                .map(|x| x.clone())
                .unwrap_or_default()
        };

        (ctx.state().rsp != self.rsp) || (ctx.state().req != self.req)
    }

    fn edge<'a>(&mut self, ctx: &super::Ctx<'a, Self>) {
        let pop = if let Some(port) = ctx.port::<RspPort<T>>(self.req.port) {
            if self.req.valid && port.ready {
                self.buffer.pop_front();
                true
            } else {
                false
            }
        } else { false };

        let pt = pop && self.zero_cycle && self.buffer.is_empty();

        if let Some(port) = ctx.port::<ReqPort<T>>(self.rsp.port) {
            if port.valid && self.rsp.ready && !pt {
                self.buffer.push_back(port.data.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::stdcells::{fifo::Fifo, sink::Sink, src::Src, Sim};

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
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

        let sink = Sink::<Msg>::new("sink".to_string());
        let fifo = Fifo::<Msg>::new(4, false);
        let src = Src::<Msg>::new(msgs.clone());
        let sink_id = sim.add_node(sink);
        let fifo_id = sim.add_node(fifo);
        let src_id = sim.add_node(src);

        let sink = sim.get_node_mut::<Sink<Msg>>(&sink_id).unwrap();
        sink.rsp.port = fifo_id.port(1);

        let fifo = sim.get_node_mut::<Fifo<Msg>>(&fifo_id).unwrap();
        fifo.rsp.port = src_id.port(0);
        fifo.req.port = sink_id.port(0);

        let src = sim.get_node_mut::<Src<Msg>>(&src_id).unwrap();
        src.req.port = fifo_id.port(0);

        sim.sim_cycle();
        sim.sim_cycle();
        sim.sim_cycle();
        sim.sim_cycle();

        // ensure that the received messages matched the ones we sent
        let sink = sim.get_node_mut::<Sink<Msg>>(&sink_id).unwrap();
        // println!("{:?}", sink.rec);
        assert_eq!(msgs[0..3], sink.rec);
    }
}
