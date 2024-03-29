use std::{marker::PhantomData};

use rand::RngCore;
use rand_chacha::ChaCha8Rng;
use tinyset::SetUsize;

#[derive(Copy, Clone)]
pub struct NodeId {
    ntype: usize,
    index: usize
}

#[derive(Copy, Clone)]
pub struct PortId {
    node: NodeId,
    port: usize
}

pub struct Ctx<'a, T> {
    comb: &'a mut T,
    reg: &'a mut T,
    first_iter: bool
}

pub trait Node : Copy {
    fn get_port_ids(&self) -> &[PortId];
    fn get_port(&self, port: usize) -> &dyn std::any::Any;
    fn lsim<'a>(ctx: Ctx<'a, Self>, sim: &Sim) -> bool;
}

trait NodeVec {
    fn len(&self) -> usize;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn dclone(&self) -> Box<dyn NodeVec>;
    fn get_port(&self, index: usize, port: usize) -> &dyn std::any::Any;
    fn lsim(&mut self, indices: &SetUsize, ntype: usize, sim: &mut Sim, first_iter: bool) -> SetUsize;
}

impl<T: 'static + Node> NodeVec for Vec<T> {
    fn len(&self) -> usize {
        self.len()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }

    fn dclone(&self) -> Box<dyn NodeVec> {
        Box::new(self.clone())
    }

    fn get_port(&self, index: usize, port: usize) -> &dyn std::any::Any {
        self.get(index).unwrap().get_port(port)
    }
    
    fn lsim(&mut self, indices: &SetUsize, ntype: usize, sim: &mut Sim, first_iter: bool) -> SetUsize {
        let reg = sim.graph[ntype].as_any_mut().downcast_mut::<Vec<T>>().unwrap();
        indices.iter()
            .map(|i| {
                let ctx = Ctx { comb: &mut self[i], reg: &mut reg[i], first_iter };
                if T::lsim(ctx, sim) {
                    // the node was modified, add it and all connected
                    // nodes to the dirty list
                    let mut ports = self[i].get_port_ids().iter()
                        .map(|pid| pid.node.index)
                        .collect::<Vec<usize>>();
                    ports.push(i);
                    ports
                } else {
                    Vec::with_capacity(0)
                }
            })
            .flatten()
            .collect::<SetUsize>()
    }
}

pub struct Sim {
    graph: Vec<Box<dyn NodeVec>>,
    rng: ChaCha8Rng,
    rng_state: u64,
}

impl Sim {
    pub fn add_node<T: 'static + Node>(&mut self, node: T) -> NodeId {
        for (i, vec) in self.graph.iter_mut().enumerate() {
            if let Some(vec) = vec
                .as_any_mut()
                .downcast_mut::<Vec<T>>() 
            {
                vec.push(node);
                return NodeId { ntype: i, index: vec.len() - 1 };
            }
        }
        let mut new_vec: Vec<T> = Vec::new();
        new_vec.push(node);
        self.graph.push(Box::new(new_vec));
        NodeId { ntype: self.graph.len() - 1, index: 0 }
    }

    pub fn get_port<T: 'static, P: 'static>(&mut self, port: &PortId) -> Option<&P> {
        self.graph.get(port.node.ntype)
            .map(|nv| nv.get_port(port.node.index, port.port))
            .map(|n| n.downcast_ref::<P>())
            .flatten()
    }

    pub fn sim_cycle(&mut self) {
        self.rng_state = self.rng.next_u64();
        let mut cpy = self.graph.iter()
            .map(|g| g.dclone())
            .collect::<Vec<Box<dyn NodeVec>>>();

        // first iteration
        for (i, vec) in cpy
            .iter_mut() 
            .enumerate()
        {
            let indices: SetUsize = (0..vec.len()).collect();
            let modified = vec.lsim(&indices, i, self, true);
            
        }
    }
}