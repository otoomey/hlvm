use std::{borrow::Cow, fmt::Debug};

use super::{reqrsp::{ReqPort, RspPort}, Node};


#[derive(Clone)]
pub struct Sink<T: Debug + Clone + 'static> {
    name: String,
    pub rec: Vec<T>,
    pub rsp: RspPort<T>
}

impl<T: Debug + Clone + 'static> Sink<T> {
    pub fn new(name: String) -> Self {
        Self {
            name,
            rec: vec![],
            rsp: RspPort::new()
        }
    }
}

impl<T: Debug + Clone + 'static> Node for Sink<T> {
    fn get_port_ids(&self) -> Cow<[super::Port]> {
        Cow::from(vec![self.rsp.port])
    }

    fn get_port(&self, _: usize) -> &dyn std::any::Any {
        &self.rsp
    }

    fn csim<'a>(&mut self, _: &super::Ctx<'a, Self>) -> bool {
        if !self.rsp.ready {
            self.rsp.ready = true;
            true
        } else {
            false
        }
    }

    fn edge<'a>(&mut self, ctx: &super::Ctx<'a, Self>) {
        if let Some(port) = ctx.port::<ReqPort<T>>(self.rsp.port) {
            if port.valid && self.rsp.ready {
                self.rec.push(port.data.clone());
                println!("[{}:{}] Received message {:?}.", self.name, ctx.cycle(), port.data);
            }
        }
        
    }
}