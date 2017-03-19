use super::super::duration::*;
use super::super::time::*;
use super::super::web::*;
use super::*;

impl ConfigNode {
    /// Set up display data for this node and all children.
    pub fn display_gantt_internal(&self,
                                  root: &ConfigNode,
                                  context: &mut TemplateContext)
                                  -> Result<(), String> {

        // Ignore "special" nodes
        for s in vec!["chart", "people", "rows"] {
            if self.data.borrow().name == format!("[{}]", s) {
                return Ok(());
            }
        }

        let weeks: u32 = try!(root.get_config_val("weeks", None));

        // Get start time for the period to allocate.  Assume that everything
        // prior to this has been committed.
        let start: ChartTime = try!(root.get_config_val("today",
                                                        Some(ChartTime::new("1").unwrap())));
        //let end: ChartTime = ChartTime::new(&format!("{}", weeks+1)).unwrap();

        // Work out which is the start week.
        let start_week = start.get_quarter() / 20;

        // Set up row data for self
        let mut row = TemplateRow::new(self.data.borrow().level,
                                       self.data.borrow().line_num,
                                       &self.data.borrow().name);
        let mut count = 0;
        for val in &self.data
                        .borrow()
                        .cells
                        .get_weekly_numbers(weeks) {
            row.add_cell(*val as f32 / 4.0, count == start_week);
            count += 1;
        }

        let done = self.data
            .borrow()
            .cells
            .count_range(0..start.get_quarter()) as f32 / 4.0;
        row.set_done(done);

        let mut plan_now = Duration::new_days(0.0);
        match self.get_plan(&ChartTime::new(&format!("{}", weeks + 1)).unwrap(),
                            &Duration::new_days(weeks as f32 * 5.0)) {
            Ok(Some(d)) => {
                plan_now = d;
                row.set_plan(d.days());
                row.set_left(d.days() - done);
            }
            Ok(None) => {}
            Err(e) => {
                self.add_note(&e);
            }
        }

        let mut plan_original = Duration::new_days(0.0);
        match self.get_plan(&ChartTime::new(&"1".to_string()).unwrap(),
                            &Duration::new_days(weeks as f32 * 5.0)) {
            Ok(Some(d)) => {
                plan_original = d;
            }
            Ok(None) => {}
            Err(e) => {
                self.add_note(&e);
            }
        }

        row.set_gain((plan_original - plan_now).days());

        for n in self.data
                .borrow()
                .notes
                .iter() {
            row.add_note(n);
        }

        let valid_who: Vec<String> = root.data
            .borrow()
            .people
            .keys()
            .map(|x| x.clone())
            .collect();

        match self.get_who(&valid_who) {
            Ok(Some(who)) => {
                row.set_who(&who);
            }
            Ok(None) => {}
            Err(e) => {
                self.add_note(&e);
            }
        };
        context.add_row(row);

        // Set up row data for children
        for child_rc in &self.children {
            try!(child_rc.borrow().display_gantt_internal(root, context));
        }

        Ok(())
    }

    /// Generate the data for displaying th gantt shart.
    ///
    /// Sets up the resource rows, then recurses throught
    /// the node hierarchy.
    pub fn display_gantt(&self, context: &mut TemplateContext) -> Result<(), String> {

        let weeks: u32 = try!(self.get_config_val("weeks", None));

        // Get start time for the period to allocate.  Assume that everything
        // prior to this has been committed.
        let start: ChartTime = try!(self.get_config_val("today",
                                                        Some(ChartTime::new("1").unwrap())));

        //let end: ChartTime = ChartTime::new(&format!("{}", weeks+1)).unwrap();

        // Work out which is the start week.
        let start_week = start.get_quarter() / 20;

        // Set up row data for people
        for (who, cells) in &self.data.borrow().people {

            let mut row = TemplateRow::new(0, 0, &who);
            let mut count = 0;
            for val in &cells.get_weekly_numbers(weeks) {
                row.add_cell(*val as f32 / 4.0, count == start_week);
                count += 1;
            }
            row.set_left(cells.count() as f32 / 4.0);
            context.add_row(row);
        }

        // Set up row data for nodes
        try!(self.display_gantt_internal(self, context));

        //Err("display_gantt is not yet implemented".to_string())
        Ok(())

    }
}
