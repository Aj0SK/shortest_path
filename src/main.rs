use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;

use std::cmp::{max, min};
use std::time::Duration;

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fs::File;

use osmpbfreader::Node;
use osmpbfreader::NodeId;
use osmpbfreader::Way;
use osmpbfreader::WayId;

use num::pow;

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 800;
const MAX_LINE_COUNT: u32 = 500_000;

const EARTH_RADIUS: f64 = 6371.0;

fn deg2rad(deg: f64) -> f64 {
    std::f64::consts::PI * deg / 180.0
}

// https://github.com/Aj0SK/mymap/blob/master/src/earthfunctions.h
fn coordinate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1 = deg2rad(lat1);
    let lon1 = deg2rad(lon1);
    let lat2 = deg2rad(lat2);
    let lon2 = deg2rad(lon2);

    let d_lat = (lat1 - lat2).abs();
    let d_lon = (lon1 - lon2).abs();

    let a = (d_lat / 2.0).sin().powf(2.0) + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powf(2.0);
    let d_sigma = 2.0 * a.sqrt().asin();
    return EARTH_RADIUS * d_sigma * 1000.0;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct NodeInfo {
    /// The tags of the node.
    pub tags: osmpbfreader::Tags,
    /// The latitude in decimicro degrees (10⁻⁷ degrees).
    pub decimicro_lat: i32,
    /// The longitude in decimicro degrees (10⁻⁷ degrees).
    pub decimicro_lon: i32,
    /// Added for easier graph implementations
    pub reachable_nodes: Vec<NodeId>,
}

impl From<&Node> for NodeInfo {
    fn from(n: &Node) -> Self {
        NodeInfo {
            tags: n.tags.clone(),
            decimicro_lat: n.decimicro_lat,
            decimicro_lon: n.decimicro_lon,
            reachable_nodes: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
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

#[derive(Debug, Clone)]
struct Map {
    nodes: HashMap<NodeId, NodeInfo>,
    ways: HashMap<WayId, WayInfo>,
}

impl Map {
    pub fn new(nodes: HashMap<NodeId, NodeInfo>, ways: HashMap<WayId, WayInfo>) -> Self {
        Self { nodes, ways }
    }

    pub fn check_connectivity(&self) -> i32 {
        let mut visited: HashMap<NodeId, bool> = HashMap::new();
        let mut to_visit: VecDeque<NodeId> = VecDeque::new();
        let mut components = 0;

        for (curr, _) in self.nodes.iter() {
            if !*visited.entry(*curr).or_insert(false)
                && self.nodes.get(&curr).unwrap().reachable_nodes.len() != 0
            {
                components += 1;
                let mut component_size = 1;
                to_visit.push_back(*curr);
                visited.insert(*curr, true);

                while !to_visit.is_empty() {
                    let node = to_visit.pop_front().unwrap();
                    component_size += 1;
                    for neigh in self.nodes.get(&node).unwrap().reachable_nodes.iter() {
                        if !*visited.entry(*neigh).or_insert(false) {
                            visited.insert(*neigh, true);
                            to_visit.push_back(*neigh);
                        }
                    }
                }
                if component_size > 500 {
                    println!("Component size is {}", component_size);
                }
            }
        }
        return components;
    }
}

struct MapDrawing {}

impl MapDrawing {
    pub fn new() -> Self {
        Self {}
    }
    pub fn draw(&self, map: Map) {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window("rust-sdl2 demo", WIDTH, HEIGHT)
            .position_centered()
            .build()
            .unwrap();
        let mut canvas = window.into_canvas().build().unwrap();
        let mut event_pump = sdl_context.event_pump().unwrap();

        'running: loop {
            canvas.set_draw_color(Color::RGB(255, 255, 255));
            canvas.clear();
            canvas.set_draw_color(Color::RGB(255, 0, 0));
            // drawing
            let mut draw_counter = 0;
            let mut to_draw: Vec<(&NodeInfo, &NodeInfo)> = Vec::new();
            for (_, way_info) in map.ways.iter() {
                draw_counter += 1;
                if draw_counter == MAX_LINE_COUNT {
                    break;
                }
                for i in 0..way_info.nodes.len() - 1 {
                    let from_id = way_info.nodes[i];
                    let to_id = way_info.nodes[i + 1];

                    let node_info_from = map.nodes.get(&from_id).unwrap();
                    let node_info_to = map.nodes.get(&to_id).unwrap();

                    to_draw.push((node_info_from, node_info_to));
                }
            }

            let mut min_lat = 1_000_000_000;
            let mut max_lat = 0;
            let mut min_lon = 1_000_000_000;
            let mut max_lon = 0;
            for (from, to) in to_draw.iter() {
                min_lat = min(min_lat, from.decimicro_lat);
                min_lon = min(min_lon, from.decimicro_lon);
                max_lat = max(max_lat, from.decimicro_lat);
                max_lon = max(max_lon, from.decimicro_lon);

                min_lat = min(min_lat, to.decimicro_lat);
                min_lon = min(min_lon, to.decimicro_lon);
                max_lat = max(max_lat, to.decimicro_lat);
                max_lon = max(max_lon, to.decimicro_lon);
            }

            let lat_diff = (max_lat - min_lat) as f64;
            let lon_diff = (max_lon - min_lon) as f64;

            for (from_node, to_node) in to_draw.iter() {
                let mut a = ((from_node.decimicro_lat - min_lat) as f64) / lat_diff;
                let mut b = ((from_node.decimicro_lon - min_lon) as f64) / lon_diff;
                let mut c = ((to_node.decimicro_lat - min_lat) as f64) / lat_diff;
                let mut d = ((to_node.decimicro_lon - min_lon) as f64) / lon_diff;

                a *= HEIGHT as f64;
                b *= WIDTH as f64;
                c *= HEIGHT as f64;
                d *= WIDTH as f64;

                let from = Point::new(b as i32, HEIGHT as i32 - (a as i32));
                let to = Point::new(d as i32, HEIGHT as i32 - (c as i32));
                canvas.draw_line(from, to).unwrap();
            }

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }

            canvas.present();
            ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        }
    }
}

fn is_highway(way: Way) -> bool {
    way.tags.into_inner().contains_key("highway")
}

fn main() {
    let f = File::open("data/slovakia-latest.osm.pbf").unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(f);

    let mut used_ids: HashSet<NodeId> = HashSet::new();
    for obj in pbf.iter() {
        if let Some(way) = obj.unwrap().way() {
            if !is_highway(way.clone()) {
                continue;
            }
            for id in way.nodes.iter() {
                used_ids.insert(*id);
            }
        }
    }
    used_ids.shrink_to_fit();

    pbf.rewind().unwrap();

    let mut nodes = HashMap::new();
    for obj in pbf.iter() {
        if let Some(node) = obj.unwrap().node() {
            if used_ids.contains(&node.id) {
                nodes.insert(node.id, NodeInfo::from(node));
            }
        }
    }

    drop(used_ids);
    pbf.rewind().unwrap();

    let mut ways: HashMap<WayId, WayInfo> = HashMap::new();
    for obj in pbf.iter() {
        if let Some(way) = obj.unwrap().way() {
            if !is_highway(way.clone()) {
                continue;
            }
            for i in 0..way.nodes.len() - 1 {
                nodes
                    .get_mut(&way.nodes[i])
                    .unwrap()
                    .reachable_nodes
                    .push(way.nodes[i + 1]);

                nodes
                    .get_mut(&way.nodes[i + 1])
                    .unwrap()
                    .reachable_nodes
                    .push(way.nodes[i]);
            }
            ways.insert(way.id, WayInfo::from(way));
        }
    }
    nodes.shrink_to_fit();
    ways.shrink_to_fit();

    let map = Map::new(nodes, ways);

    println!("Number of components is {}", map.check_connectivity());

    let draw = MapDrawing::new();
    draw.draw(map);
}
