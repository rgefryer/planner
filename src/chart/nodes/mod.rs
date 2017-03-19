mod generate;
mod allocate;
mod display;
mod derive;
mod management;

use std::collections::HashMap;
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::str::FromStr;
use std::rc::Rc;
use std::rc::Weak;
use std::fmt::Display;
use super::time::*;
use super::timerow::*;

#[derive(Debug)]
struct ConfigNodeData {
    name: String,
    line_num: u32,
    indent: u32,
    level: u32, // Root node is level 0
    attributes: HashMap<String, String>,

    // People are only defined on the root node
    people: HashMap<String, ChartTimeRow>,

    // Cells are only used on leaf nodes
    cells: ChartTimeRow,

    // Optional first and last dates that the task
    // can happen on.
    start: Option<ChartTime>,
    end: Option<ChartTime>,

    // Notes are problems to display on the chart
    notes: Vec<String>,
}

impl ConfigNodeData {
    fn new(name: &str, level: u32, indent: u32, line_num: u32) -> ConfigNodeData {
        ConfigNodeData {
            name: name.to_string(),
            line_num: line_num,
            indent: indent,
            level: level,
            attributes: HashMap::new(),
            people: HashMap::new(),
            cells: ChartTimeRow::new(),
            start: None,
            end: None,
            notes: Vec::new(),
        }

    }

    fn add_note(&mut self, note: &str) {

        self.notes.push(note.to_string());
    }

    fn update_start(&mut self, start: ChartTime) {

        // Move out the start date if necessary
        match self.start {
            Some(t) => {
                if start > t {
                    self.start = Some(start);
                }
            }
            None => {
                self.start = Some(start);
            }
        };
    }

    fn update_end(&mut self, end: ChartTime) {

        // Move in the end date if necessary
        match self.end {
            Some(t) => {
                if end < t {
                    self.end = Some(end);
                }
            }
            None => {
                self.end = Some(end);
            }
        };
    }
}

#[derive(Debug)]
pub struct ConfigNode {
    children: Vec<Rc<RefCell<ConfigNode>>>,

    parent: Option<Weak<RefCell<ConfigNode>>>,

    data: RefCell<ConfigNodeData>,
}

impl ConfigNode {
    pub fn new(name: &str, level: u32, indent: u32, line_num: u32) -> ConfigNode {
        ConfigNode {
            data: RefCell::new(ConfigNodeData::new(name, level, indent, line_num)),
            children: Vec::new(),
            parent: None,
        }
    }

    fn create_attribute(&self, key: &str, val: &str) {
        self.data
            .borrow_mut()
            .attributes
            .insert(key.to_string(), val.to_string());
    }

    fn new_child(&mut self, name: &str, indent: u32, line_num: u32) {
        self.children.push(Rc::new(RefCell::new(ConfigNode::new(name,
                                                                self.data.borrow().level + 1,
                                                                indent,
                                                                line_num))));
    }

    pub fn count_nodes(&self) -> u32 {
        let mut count = 1u32;

        for child_rc in &self.children {
            count += child_rc.borrow().count_nodes();
        }

        count
    }

    pub fn count_children(&self) -> u32 {
        let mut count = 0u32;

        for _ in &self.children {
            count += 1;
        }

        count
    }

    pub fn find_child_with_name(&self, name: &str) -> Option<Weak<RefCell<ConfigNode>>> {

        if self.data.borrow().name == name {
            return None;
        } else {
            for child_rc in &self.children {
                let child_node = child_rc.borrow();
                if child_node.data.borrow().name == name {
                    return Some(Rc::downgrade(child_rc));
                }
            }
        }

        None
    }

    pub fn get_weak_ref(&self) -> Option<Weak<RefCell<ConfigNode>>> {

        match self.parent {
            Some(ref p) => {
                let parent = p.upgrade().unwrap();
                for child_rc in &parent.borrow().children {
                    let child_node = child_rc.borrow();
                    if child_node.data.borrow().line_num == self.data.borrow().line_num {
                        return Some(Rc::downgrade(child_rc));
                    }
                }
                return None;
            }
            None => {
                return None;
            }
        };

    }


    /// Return true if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.children.len() == 0
    }


    /// Get new hashmap containing all of the attributes
    pub fn get_attribute_hash(&self) -> HashMap<String, String> {
        let mut attr_hash = HashMap::new();

        for (key, val) in self.data
                .borrow()
                .attributes
                .iter() {
            attr_hash.insert(key.clone(), val.clone());
        }

        attr_hash
    }

    /// Get a copy of the attributes on the [chart] node
    ///
    /// This must only be called on the root node.
    pub fn get_global_config(&self) -> HashMap<String, String> {
        match self.find_child_with_name("[chart]") {
            None => HashMap::new(),
            Some(node) => {
                match node.upgrade() {
                    None => HashMap::new(),
                    Some(n) => n.borrow().get_attribute_hash(),
                }
            }
        }
    }

    fn max_time(&self, a: Option<ChartTime>, b: Option<u32>) -> Option<ChartTime> {

        match b {
            Some(b_q) => {
                let b_ct = ChartTime::new_from_quarter(b_q);
                match a {
                    Some(a_ct) => if b_ct > a_ct { Some(b_ct) } else { Some(a_ct) },
                    None => Some(b_ct),
                }
            }
            None => a,
        }
    }

    fn max_time_ct(&self, a: Option<ChartTime>, b: Option<ChartTime>) -> Option<ChartTime> {

        match b {
            Some(b_ct) => {
                match a {
                    Some(a_ct) => if b_ct > a_ct { Some(b_ct) } else { Some(a_ct) },
                    None => Some(b_ct),
                }
            }
            None => a,
        }
    }

    /// Add a note to be displayed alongside this cell
    fn add_note(&self, note: &str) {

        self.data.borrow_mut().add_note(note);
    }

    // @@@ Derive Remaining and slip/gain

    fn augment_error(&self, err: String) -> String {
        format!("Problem in node at line {}: {}",
                self.data.borrow().line_num,
                err.to_string())

    }


    /// Return a weak reference to a child Node at a given line in the
    /// config file.
    pub fn get_node_at_line(&self, line_num: u32) -> Option<Weak<RefCell<ConfigNode>>> {

        if self.data.borrow().line_num == line_num {
            return None;
        } else {
            for child_rc in &self.children {
                let child = child_rc.borrow();
                if child.data.borrow().line_num == line_num {
                    return Some(Rc::downgrade(child_rc));
                } else {
                    match child_rc.borrow().get_node_at_line(line_num) {
                        Some(x) => return Some(x),
                        None => {}
                    };
                }
            }
        }

        None
    }
}
