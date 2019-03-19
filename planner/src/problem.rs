use rolling::input::staticinfrastructure::{NodeId, RouteEntryExit};
use crate::movement::*;
use std::collections::HashMap;


// Plan interface

pub struct Config {
    pub n_before: usize,
    pub n_after: usize,
}

pub type PartialRouteId = (usize,usize); // Route index and partial index into route
pub type OverlapId = usize;
pub type TrainId = usize;

pub struct PartialRoute {
    pub entry: RouteEntryExit,
    pub exit: RouteEntryExit,
    pub conflicts: Vec<Vec<(PartialRouteId, usize)>>, // ??
    pub wait_conflict :Option<OverlapId>,
    pub contains_nodes :Vec<NodeId>,
    pub length: f32,
}

type ElementaryRoute = Vec<PartialRouteId>;

pub struct Train {
    pub length: f32,
    pub visits: Vec<Vec<NodeId>>,
    pub vehicle: Vehicle,
}

pub struct TrainOrd {
    pub a :(TrainId, VisitId),
    pub b :(TrainId, VisitId),
}

pub struct Problem {
    pub partial_routes: HashMap<PartialRouteId, PartialRoute>,
    pub elementary_routes: Vec<ElementaryRoute>,
    pub trains: HashMap<TrainId,Train>,
    pub train_ord: Vec<TrainOrd>,
}


// OUTPUT: route plan

pub type RoutePlan = Vec<Vec<(PartialRouteId, Option<TrainId>)>>;

