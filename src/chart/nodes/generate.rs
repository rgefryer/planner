use super::super::file::*;
use super::*;

impl ConfigNode {
    /// Read the config and build up a node hierarchy
    ///
    /// This function is recursive
    /// - Child nodes are created, then given control
    /// - Higher-level nodes are passed to the parent to deal with
    pub fn consume_config(&mut self,
                          ref_self: Option<&Rc<RefCell<ConfigNode>>>,
                          file: &mut ConfigLines)
                          -> Result<(), String> {

        // Loop through the config, handling nodes and attributes differently
        loop {
            match file.peek_line() {
                Some(Line::Node(LineNode { line_num, indent, name })) => {

                    // Higher-level or sibling node - return to the parent
                    // to handle.
                    if self.data.borrow().indent >= indent {
                        break;
                    }

                    // Create new child
                    file.get_line().unwrap();
                    self.new_child(&name, indent, line_num);

                    // Set child's back-pointer as a weak reference to self
                    let new_child = &self.children[self.children.len() - 1];
                    new_child.borrow_mut().parent = match ref_self {
                        None => None,
                        Some(rc) => Some(Rc::downgrade(rc)),
                    };

                    // Pass control to the child
                    new_child.borrow_mut().consume_config(Some(&new_child), file).unwrap();
                }

                // Attributes are simply added to the current node.
                Some(Line::Attribute(LineAttribute { key, value })) => {
                    self.create_attribute(&key, &value);
                    file.get_line().unwrap();
                }

                // End of config
                None => {
                    break;
                }
            };
        }

        Ok(())
    }
}
