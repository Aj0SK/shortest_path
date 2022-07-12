use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fs::File;
use std::process::exit;

use osmpbfreader::Node;
use osmpbfreader::NodeId;
use osmpbfreader::Way;
use osmpbfreader::WayId;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct NodeInfo {
    /// The tags of the node.
    pub tags: osmpbfreader::Tags,
    /// The latitude in decimicro degrees (10⁻⁷ degrees).
    pub decimicro_lat: i32,
    /// The longitude in decimicro degrees (10⁻⁷ degrees).
    pub decimicro_lon: i32,
    /// Added for easier graph implementations
    pub reachable_ways: Vec<WayId>,
}

impl From<&Node> for NodeInfo {
    fn from(n: &Node) -> Self {
        NodeInfo {
            tags: n.tags.clone(),
            decimicro_lat: n.decimicro_lat,
            decimicro_lon: n.decimicro_lon,
            reachable_ways: Vec::new(),
        }
    }
}

pub struct WayInfo {
    /// The tags of the way.
    pub tags: osmpbfreader::Tags,
    /// The ordered list of nodes as id.
    pub nodes: Vec<osmpbfreader::NodeId>,
}

impl From<&Way> for WayInfo {
    fn from(n: &Way) -> Self {
        WayInfo {
            tags: n.tags.clone(),
            nodes: n.nodes.clone(),
        }
    }
}

fn check_connectivity(nodes: HashMap<NodeId, NodeInfo>, ways: HashMap<WayId, WayInfo>) {
    let mut visited: HashMap<NodeId, bool> = HashMap::new();
    let mut to_visit: VecDeque<NodeId> = VecDeque::new();

    let mut components = 0;

    for (curr, _) in nodes.iter() {
        if !*visited.entry(*curr).or_insert(false)
            && nodes.get(&curr).unwrap().reachable_ways.len() != 0
        {
            components += 1;
            let mut component_size = 1;
            to_visit.push_back(*curr);
            visited.insert(*curr, true);

            while !to_visit.is_empty() {
                let node = to_visit.pop_front().unwrap();
                component_size += 1;
                for way in nodes.get(&node).unwrap().reachable_ways.iter() {
                    for neigh in ways.get(&way).unwrap().nodes.iter() {
                        if !*visited.entry(*neigh).or_insert(false) {
                            visited.insert(*neigh, true);
                            to_visit.push_back(*neigh);
                        }
                    }
                }
            }
            if component_size > 500 {
                println!("Component size is {}", component_size);
            }
        }
    }
    println!("Number of components is {}", components);
}

fn main() {
    let f = File::open("data/slovakia-latest.osm.pbf").unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(f);

    let mut used_ids: HashSet<NodeId> = HashSet::new();

    for obj in pbf.iter() {
        // error handling:
        let obj = obj.unwrap_or_else(|e| {
            println!("{:?}", e);
            exit(1)
        });
        if let Some(way) = obj.way() {
            for id in way.nodes.iter() {
                used_ids.insert(*id);
            }
        }
    }

    used_ids.shrink_to_fit();

    let mut nodes = HashMap::new();

    pbf.rewind().unwrap();
    for obj in pbf.iter() {
        if let Some(node) = obj.unwrap().node() {
            if used_ids.contains(&node.id) {
                nodes.insert(node.id, NodeInfo::from(node));
            }
        }
    }

    drop(used_ids);

    let mut ways: HashMap<WayId, WayInfo> = HashMap::new();

    pbf.rewind().unwrap();
    for obj in pbf.iter() {
        // error handling:
        let obj = obj.unwrap_or_else(|e| {
            println!("{:?}", e);
            exit(1)
        });
        if let Some(way) = obj.way() {
            for id in way.nodes.iter() {
                nodes.get_mut(&id).unwrap().reachable_ways.push(way.id);
            }
            ways.insert(way.id, WayInfo::from(way));
        }
    }

    nodes.shrink_to_fit();
    ways.shrink_to_fit();

    check_connectivity(nodes, ways);
}
