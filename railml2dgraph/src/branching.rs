use minidom; 
use std::collections::HashMap;
use std::collections::HashSet;
use base::*;

pub struct BranchingModel {
    pub tracks:       Vec<BrTrack>,
    pub connections:  HashMap<(String,String), BrCursor>,
    pub switch_conns: HashSet<(String,String)>,
}

#[derive(Clone)]
pub struct BrTrack {
    pub name: String,
    pub begin :BrTrackEnd,
    pub objs :Vec<BrObject>,
    pub length: f64,
    pub end: BrTrackEnd,
}

#[derive(Copy, Clone, Debug)]
pub struct BrCursor {
    pub track: usize,
    pub offset: f64,
    pub dir: Dir,
}

#[derive(Clone, Debug)]
pub enum BrTrackEnd {
    Stop,
    Boundary(String),
    Connection((String,String)),
}

#[derive(Clone)]
pub struct BrObject {
    pub name :String,
    pub pos: f64,
    pub data :BrObjectData,
}

#[derive(Clone)]
pub enum BrObjectData {
    Signal { dir: Dir, sight: f64 },
    Sight { dir: Dir, signal: String, distance: f64 },
    Detector,
    Switch { dir :Dir, side: Side, conn: (String,String) }, // TODO what about when continuing track is the branching track?
}


pub fn get_branching_model<'a>(doc :&'a minidom::Element, ns: &str) 
    -> Result<BranchingModel, String> {

    // Convert railML from XML into a similar structure that we
    // call "branching model", meaning that tracks can have branches (switches
    // or crossings) in the middle. Contrast this to more graph-like models
    // such as railtopomodel (linear/non-branching track sections connected by relations)
    // or the doppelpunktgraph (double/twin nodes giving local directionality, 
    // and no objects between nodes)

    let mut model = BranchingModel {
        tracks: Vec::new(),
        connections: HashMap::new(),
        switch_conns: HashSet::new(),
    };

    let infrastructure = {
        if doc.name().to_lowercase() == "infrastructure" {
            doc
        } else if doc.name().to_lowercase() == "railml" {
            doc.children()
                .filter(|x| x.name().to_lowercase() == "infrastructure")
                .nth(0)
                .ok_or(format!("No infrastructure element found"))?
        } else {
            return Err(format!("No infrastructure element found"));
        }
    };

    let tracks = infrastructure.children()
        .filter(|x| x.name().to_lowercase() == "tracks")
        .nth(0)
        .ok_or(format!("No tracks found in the infrastructure"))?;

    //let conn_id_to_cursor = HashMap::new();
    //let conn_ref_to_id = HashMap::new();

    for track in tracks.children().filter(|x| x.name().to_lowercase() == "track") {
        let track_idx = model.tracks.len();
        let name = track.attr("name")
            .ok_or(format!("No name for track"))?;

        let topology = track.get_child("trackTopology", ns)
            .ok_or(format!("No trackTopology element in track {:?}", name))?;

        let track_length = topology.get_child("trackEnd", ns)
            .ok_or(format!("No trackEnd element on track {:?}", name))?
            .attr("pos").ok_or(format!("No pos attribute on trackEnd on track {:?}", name))?
            .parse::<f64>().map_err(|e| format!("{:?}", e))?;

        let track_begin = topology.get_child("trackBegin", ns)
            .ok_or(format!("No trackBegin found on track {:?}.", name))?;
        let track_end = topology.get_child("trackEnd", ns)
            .ok_or(format!("No trackEnd found on track {:?}.", name))?;

        let (begin,end) = {
            let mut conv_end = |n:&minidom::Element,offset,dir| {
                if let Some(conn) = n.get_child("connection", ns) {
                    let id = conn.attr("id").ok_or(format!("Connection has no id.")).unwrap();
                    let ref_ = conn.attr("ref").ok_or(format!("Connection has no ref.")).unwrap();
                    model.connections.insert((ref_.to_string(),id.to_string()), 
                                             BrCursor { track: track_idx, offset, dir });
                    BrTrackEnd::Connection((id.to_string(),ref_.to_string()))
                } else if let Some(e) = n.get_child("openEnd", ns) {
                    let name = e.attr("id").ok_or(format!("openEnd has no id.")).unwrap();
                    BrTrackEnd::Boundary(name.to_string())
                } else {
                    BrTrackEnd::Stop
                }
            };
            let begin = conv_end(track_begin, 0.0, Dir::Up);
            let end   = conv_end(track_end, track_length, Dir::Down);
            (begin,end)
        };

        let mut objs = Vec::new();
        add_signals(&mut objs, &track, ns);
        add_detectors(&mut objs, &track, ns);
        add_switches(&mut objs, &mut model.connections, &mut model.switch_conns,
                     &track, track_idx, ns);

        model.tracks.push(BrTrack { name: name.to_string(), begin, objs, length: track_length, end });
    }

    Ok(model)
}

fn add_signals(vec :&mut Vec<BrObject>, track :&minidom::Element, ns :&str) {
    let signal_elements = track.get_child("ocsElements", ns)
        .and_then(|o| o.get_child("signals", ns))
        .map(|s| s.children().filter(|x| x.name() == "signal").collect())
        .unwrap_or_else(|| Vec::new());

    for s in signal_elements {
        let id = s.attr("id").expect("signal id missing");
        //let _name = s.attr("name").expect("signal name missing");
        let name = id.to_string();
        let pos = s.attr("pos").expect("signal pos missing")
            .parse::<f64>().expect("signal pos is not a valid number");
        let sight = s.attr("sight")
            .map(|x| x.parse::<f64>().expect("signal sight is not a valid number"))
            .unwrap_or_else(|| {
                println!("Warning: no sight info for signal {:?}", name);
                200.0
            });
        let dir = match s.attr("dir") {
            Some("up") => Dir::Up,
            Some("down") => Dir::Down,
            _ => {
                continue;
            }
        };

        let relevant_type = match s.attr("type") {
            Some("main") | Some("combined") => true,
            Some(_) => false,
            None => true, // TODO warning?
        };

        if relevant_type {
            vec.push(BrObject { name, pos, data: BrObjectData::Signal { dir, sight } });
        }
    }
}

fn add_detectors(vec :&mut Vec<BrObject>, track :&minidom::Element, ns :&str) {
    let detector_elements = track.get_child("ocsElements", ns)
        .and_then(|o| o.get_child("trainDetectionElements", ns))
        .map(|s| {
            s.children()
                .filter(|x| x.name() == "trainDetector" || x.name() == "trackCircuitBorder")
                .collect()
        })
        .unwrap_or_else(|| Vec::new());

    for d in &detector_elements {
        let id = d.attr("id").expect("detector id missing");
        let name = id.to_string();
        let pos = d.attr("pos").expect("detector pos missing")
            .parse::<f64>().expect("detector pos is not a valid number");

        match d.attr("dir") {
            Some("up") | Some("down") => {
                println!("Warning: directional detector is not supported {:?}", name);
            }
            _ => {},
        };

        vec.push(BrObject { name, pos, data: BrObjectData::Detector });
    }
}

fn add_switches(vec :&mut Vec<BrObject>, connections :&mut HashMap<(String,String), BrCursor>, switch_conns :&mut HashSet<(String,String)>, track :&minidom::Element, track_idx: usize, ns :&str) {
    let switches = track.get_child("trackTopology",ns)
        .and_then(|o| o.get_child("connections", ns))
        .map(|c| {
            c.children()
                .filter(|x| x.name() == "switch" /*|| TODO x.name() == "crossing"*/)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| Vec::new());


    for sw in &switches {
        let id = sw.attr("id").expect("switch id missing");
        let name = id.to_string();
        let pos = sw.attr("pos").expect("switch pos missing")
        .parse::<f64>().expect("switch pos is not a valid number");

        let conns = sw.children().filter(|x| x.name() == "connection").collect::<Vec<_>>();
        if conns.len() == 0 {
            println!("No connection in switch {:?}", name);
            panic!();
        }
        if conns.len() > 1 {
            println!("Multiple connections in switch {:?}", name);
            panic!();
        }

        let conn = conns[0];
        let dir = match conn.attr("orientation") {
            Some("outgoing") => Dir::Up,
            Some("incoming") => Dir::Down,
            _ => panic!("Switch orientation"),
        };

        let side = match conn.attr("course") {
            Some("left") => Side::Left,
            Some("right") => Side::Right,
            _ => panic!("Switch course"),
        };

        let conn_name = match (conn.attr("id"), conn.attr("ref")) {
            (Some(id),Some(ref_)) => (id.to_string(), ref_.to_string()),
            _ => panic!("Switch connection ids missing"),
        };

        connections.insert((conn_name.1.clone(), conn_name.0.clone()), 
                           BrCursor { track: track_idx, offset: pos, dir: dir.opposite() });
        switch_conns.insert((conn_name.0.clone(), conn_name.1.clone()));
        switch_conns.insert((conn_name.1.clone(), conn_name.0.clone()));
        vec.push(BrObject { name, pos, data: BrObjectData::Switch { dir, side, conn: conn_name }});
    }
}


#[derive(Debug)]
pub enum WalkResult {
    Ok(BrCursor),
    End(f64, BrCursor),
    TrailingSwitch(f64, BrCursor, BrCursor),
    FacingSwitch(f64, BrCursor, BrCursor, BrCursor),
}

pub fn walk(m :&BranchingModel, cursor :&BrCursor, dist: f64, delta :f64) -> WalkResult {
    let track = &m.tracks[cursor.track];
    let mut objs = track.objs.clone();
    println!("WAlk from {:?}", cursor);
    match cursor.dir {
        Dir::Up => {
            objs.sort_by(|a,b| a.pos.partial_cmp(&b.pos).unwrap());
            for obj in &objs {
                if obj.pos < cursor.offset { continue; }
                if obj.pos > cursor.offset + dist {
                    return WalkResult::Ok(BrCursor {
                        track: cursor.track,
                        offset: cursor.offset + dist,
                        dir: cursor.dir,
                    });
                }
                match &obj.data {
                    BrObjectData::Switch { dir: Dir::Up, conn, .. } => {
                        let c1 = BrCursor {
                            track: cursor.track,
                            offset: obj.pos - delta,
                            dir: cursor.dir,
                        };
                        let c2 = BrCursor {
                            track: cursor.track,
                            offset: obj.pos + delta,
                            dir: cursor.dir,
                        };
                        let c3 = m.connections[&conn].clone();
                        return WalkResult::FacingSwitch((obj.pos - cursor.offset).abs(), c1, c2, c3);
                    },
                    BrObjectData::Switch { dir: Dir::Down, .. } => {
                        let c1 = BrCursor {
                            track: cursor.track,
                            offset: obj.pos - delta,
                            dir: cursor.dir,
                        };
                        let c2 = BrCursor {
                            track: cursor.track,
                            offset: obj.pos + delta,
                            dir: cursor.dir,
                        };
                        return WalkResult::TrailingSwitch((obj.pos - cursor.offset).abs(), c1,c2);
                    },
                    _ => {},
                }
            }

            if m.tracks[cursor.track].length - cursor.offset > dist {
                return WalkResult::Ok(BrCursor {
                    track: cursor.track,
                    offset: cursor.offset + dist,
                    dir: cursor.dir});
            } else {
                let end = BrCursor {
                   track: cursor.track,
                   offset: m.tracks[cursor.track].length - delta,
                   dir: cursor.dir,
                };
                match &m.tracks[cursor.track].end {
                    BrTrackEnd::Stop | BrTrackEnd::Boundary(_) => {
                        return WalkResult::End((m.tracks[cursor.track].length - cursor.offset).abs(), end);
                    },
                    BrTrackEnd::Connection(c) => {
                        // Connection on track end must be a continuation or a trailing switch
                        match m.connections.get(&c) {
                            Some(after) => {
                                if m.switch_conns.contains(&c) {
                                    return WalkResult::TrailingSwitch((m.tracks[cursor.track].length - cursor.offset).abs(), end, *after);
                                } else {
                                    return walk(m, after, dist - (m.tracks[cursor.track].length - cursor.offset).abs(), delta);
                                }
                            }
                            None => {
                                return WalkResult::End((m.tracks[cursor.track].length - cursor.offset).abs(), end);
                            }
                        }
                    }
                }
            }
        }
        Dir::Down => {
            objs.sort_by(|a,b| (-a.pos).partial_cmp(&(-b.pos)).unwrap());
            for obj in &objs {
                if obj.pos > cursor.offset { continue; }
                if obj.pos < cursor.offset - dist {
                    return WalkResult::Ok(BrCursor {
                        track: cursor.track,
                        offset: cursor.offset - dist,
                        dir: cursor.dir,
                    });
                }
                match &obj.data {
                    BrObjectData::Switch { dir: Dir::Down, conn, .. } => {
                        let c1 = BrCursor {
                            track: cursor.track,
                            offset: obj.pos + delta,
                            dir: cursor.dir,
                        };
                        let c2 = BrCursor {
                            track: cursor.track,
                            offset: obj.pos - delta,
                            dir: cursor.dir,
                        };
                        let c3 = m.connections[&conn].clone();
                        return WalkResult::FacingSwitch((obj.pos - cursor.offset).abs(), c1, c2, c3);
                    },
                    BrObjectData::Switch { dir: Dir::Up, .. } => {
                        let c1 = BrCursor {
                            track: cursor.track,
                            offset: obj.pos - delta,
                            dir: cursor.dir,
                        };
                        let c2 = BrCursor {
                            track: cursor.track,
                            offset: obj.pos + delta,
                            dir: cursor.dir,
                        };
                        return WalkResult::TrailingSwitch((obj.pos - cursor.offset).abs(), c1,c2);
                    },
                    _ => {},
                }
            }

            if cursor.offset - 0.0 > dist {
                return WalkResult::Ok(BrCursor {
                    track: cursor.track,
                    offset: cursor.offset - dist,
                    dir: cursor.dir});
            } else {
                let end = BrCursor {
                                                   track: cursor.track,
                                                   offset: delta,
                                                   dir: cursor.dir,
                                               };
                match &m.tracks[cursor.track].begin {
                    BrTrackEnd::Stop | BrTrackEnd::Boundary(_) => {
                        return WalkResult::End((0.0 - cursor.offset).abs(), end);
                    },
                    BrTrackEnd::Connection(c) => {
                        match m.connections.get(&c) {
                            Some(after) => {
                                if m.switch_conns.contains(&c) {
                                    // Trailing switch over track begin connection
                                    return WalkResult::TrailingSwitch((0.0 - cursor.offset).abs(), end, *after);
                                } else {
                                    return walk(m, after, dist - (0.0 - cursor.offset).abs(), delta);
                                }
                            }
                            None =>  {
                                return WalkResult::End((0.0 - cursor.offset).abs(), end);
                            }
                        }
                    }
                }
            }

        }
    }
}
