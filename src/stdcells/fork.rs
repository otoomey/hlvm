use std::borrow::Cow;

use super::{reqrsp::{ReqPort, RspPort}, Node};

#[derive(Clone)]
pub struct Fork<T: Clone + Eq + PartialEq + Default + 'static> {
    waiting: Vec<bool>,
    pub rsp: RspPort<T>,
    pub req: Vec<ReqPort<T>>
}

impl<T: Clone + 'static + Eq + PartialEq + Default> Fork<T> {
    pub fn new(num_outs: usize) -> Self {
        Self {
            buffer: None,
            waiting: vec![true; num_outs],
            rsp: RspPort::new(),
            req: vec![ReqPort::new(Default::default()); num_outs]
        }
    }
}

impl<T: Clone + Eq + PartialEq + Default + 'static> Node for Fork<T> {
    fn get_port_ids(&self) -> std::borrow::Cow<[super::Port]> {
        let mut v = Vec::with_capacity(self.req.len() + 1);
        v.push(self.rsp.port);
        v.extend(self.req.iter().map(|r| r.port));
        Cow::from(v)
    }

    fn get_port(&self, port: usize) -> &dyn std::any::Any {
        match port {
            0 => &self.rsp,
            _ => &self.req[port-1]
        }
    }

    fn csim<'a>(&mut self, ctx: &super::Ctx<'a, Self>) -> bool {
        let ivalid = ctx.port::<ReqPort<T>>(self.rsp.port)
            .map(|req| req.valid)
            .unwrap_or(false);

        for (r, &wait) in self.req.iter_mut().zip(self.waiting.iter()) {
            r.valid = wait && ivalid;
        }
        self.rsp.ready = self.waiting.iter().all(|w| *w);

        true
    }

    fn edge<'a>(&mut self, ctx: &super::Ctx<'a, Self>) {
        todo!()
    }
}