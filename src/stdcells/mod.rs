use std::{borrow::Cow, marker::PhantomData};

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use tinyset::SetUsize;

pub mod reqrsp;
pub mod src;
pub mod sink;
pub mod fifo;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct NodeId {
    ntype: usize,
    index: usize
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Port {
    Wire {
        node: NodeId,
        port: usize
    },
    Z
}

pub struct Ctx<'a, T: 'static> {
    sim: &'a Sim,
    id: NodeId,
    phantom: PhantomData<&'a T>
}

pub struct Sim {
    graph: Vec<Box<dyn NodeVec>>,
    rng: ChaCha8Rng,
    rng_state: u64,
    cycle: u64
}

pub trait Node : Clone {
    fn get_port_ids(&self) -> Cow<[Port]>;
    fn get_port(&self, port: usize) -> &dyn std::any::Any;
    fn csim<'a>(&mut self, ctx: &Ctx<'a, Self>) -> bool;
    fn edge<'a>(&mut self, ctx: &Ctx<'a, Self>);
}

trait NodeVec {
    fn len(&self) -> usize;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn dclone(&self) -> Box<dyn NodeVec>;
    fn get_port(&self, index: usize, port: usize) -> &dyn std::any::Any;
    fn csim(&mut self, ntype: usize, indices: &SetUsize, sim: &Sim) -> SetUsize;
    fn edge(&mut self, ntype: usize, sim: &Sim);
}

impl NodeId {
    fn port(self, port: usize) -> Port {
        Port::Wire { node: self, port }
    }
}

impl Port {
    fn dst_node(&self) -> Option<NodeId> {
        if let Port::Wire { node, port } = self {
            return Some(*node)
        } else {
            return None
        }
    }

    fn dst_port(&self) -> Option<usize> {
        if let Port::Wire { node, port } = self {
            return Some(*port)
        } else {
            return None
        }
    }

    fn dst(&self) -> Option<(NodeId, usize)> {
        if let Port::Wire { node, port } = self {
            return Some((*node, *port))
        } else {
            return None
        }
    }
}

impl<'a, T> Ctx<'a, T> {
    pub fn state(&self) -> &'a T {
        let vec = self.sim.graph[self.id.ntype]
            .as_any()
            .downcast_ref::<Vec<T>>()
            .unwrap();
        &vec[self.id.index]
    }

    pub fn port<P: 'static>(&self, port: Port) -> Option<&'a P> {
        if let Some((node, pid)) = port.dst() {
            self.sim.graph[node.ntype]
                .get_port(node.index, pid)
                .downcast_ref::<P>()
        } else {
            None
        }
    }

    pub fn rng(&self) -> ChaCha8Rng {
        let seed = (self.id.ntype + self.id.index << 16) as u64;
        let seed = seed ^ self.sim.rng_state;
        ChaCha8Rng::seed_from_u64(seed)
    }

    pub fn cycle(&self) -> u64 {
        self.sim.cycle
    }
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
    
    fn csim(&mut self, ntype: usize, indices: &SetUsize, sim: &Sim) -> SetUsize {
        indices.iter()
            .map(|index| {
                let ctx = Ctx { 
                    sim,
                    id: NodeId { ntype, index },
                    phantom: PhantomData
                };
                if self[index].csim(&ctx) {
                    // the node was modified, add it and all connected
                    // nodes to the dirty list
                    let mut ports = self[index].get_port_ids().iter()
                        .map(|port| port.dst())
                        .flatten()
                        .map(|(node, pid)| node.index)
                        .collect::<Vec<usize>>();
                    ports.push(index);
                    ports
                } else {
                    Vec::with_capacity(0)
                }
            })
            .flatten()
            .collect::<SetUsize>()
    }
    
    fn edge(&mut self, ntype: usize, sim: &Sim) {
        for (index, n) in self.iter_mut().enumerate() {
            let ctx = Ctx { 
                sim,
                id: NodeId { ntype, index },
                phantom: PhantomData
            };
            n.edge(&ctx)
        }
    }
}

impl Sim {
    pub fn new(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let rng_state = rng.next_u64();
        Self {
            graph: Vec::new(),
            rng,
            rng_state,
            cycle: 0
        }
    }

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

    pub fn get_node<T: 'static + Node>(&self, id: &NodeId) -> Option<&T> {
        self.graph.get(id.ntype)
            .map(|vec| vec.as_any().downcast_ref::<Vec<T>>())
            .flatten()
            .map(|vec| vec.get(id.index))
            .flatten()
    }

    pub fn get_node_mut<T: 'static + Node>(&mut self, id: &NodeId) -> Option<&mut T> {
        self.graph.get_mut(id.ntype)
            .map(|vec| vec.as_any_mut().downcast_mut::<Vec<T>>())
            .flatten()
            .map(|vec| vec.get_mut(id.index))
            .flatten()
    }

    // pub fn get_port<T: 'static, P: 'static>(&mut self, port: &Port) -> Option<&P> {
    //     self.graph.get(port.node.ntype)
    //         .map(|nv| nv.get_port(port.node.index, port.port))
    //         .map(|n| n.downcast_ref::<P>())
    //         .flatten()
    // }

    pub fn sim_cycle(&mut self) {
        self.rng_state = self.rng.next_u64();
        let mut index_vec: Vec<SetUsize> = (0..self.graph.len())
            .map(|i| (0..self.graph[i].len()).collect())
            .collect();
        while index_vec.iter().any(|s| !s.is_empty()) {
            let mut cpy = self.graph.iter()
                .map(|g| g.dclone())
                .collect::<Vec<Box<dyn NodeVec>>>();
            for (i, indices) in index_vec.iter_mut()
                .filter(|s| !s.is_empty())
                .enumerate() 
            {
                *indices = cpy[i].csim(i, &indices, self);
            }
            self.graph = cpy;
        }

        self.cycle += 1;

        let mut cpy = self.graph.iter()
            .map(|g| g.dclone())
            .collect::<Vec<Box<dyn NodeVec>>>();
        for (i, vec) in cpy
            .iter_mut() 
            .enumerate()
        {
            vec.edge(i, self);
        }
        self.graph = cpy;
        println!("");
    }
}