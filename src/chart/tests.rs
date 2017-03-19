use super::duration::*;
use super::time::*;
use super::timerow::*;
use super::nodes::*;
use super::file::*;
use super::SchedulingStrategy;
use super::ResourcingStrategy;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

#[test]
fn duration_test() {
    let d = Duration::new_days(1.25);
    assert!(d.quarters() == 5);

    let mut dm = d;
    dm.add_days(2.25);
    dm.remove_days(0.75);
    dm.add_quarters(3);
    dm.remove_quarters(2);
    assert!(dm.quarters() == 12);

    dm.add_quarters(1);
    assert!(dm.days() > 3.249);
    assert!(dm.days() < 3.251);

    let e = Duration::new_quarters(13);
    assert!(e == dm);

    let two_weeks = Duration::new_days(10.0f32);
    let f = Duration::new_from_string("13.25", &two_weeks).unwrap();
    assert!(f.quarters() == 53);

    let g = Duration::new_from_string("4pcm", &two_weeks).unwrap();
    assert!(g.quarters() == 8);

    let h = Duration::new_from_string("52pcy", &two_weeks).unwrap();
    assert!(h.quarters() == 8);
}

#[test]
fn time_test() {
    let t1 = ChartTime::new("3").unwrap();
    let t2 = ChartTime::new("3.2").unwrap();
    let t3 = ChartTime::new("3.2.4").unwrap();

    assert_eq!(t1.get_quarter(), 40);
    assert_eq!(t2.get_quarter(), 44);
    assert_eq!(t3.get_quarter(), 47);

    assert_eq!(t1.get_duration().quarters(), 20);
    assert_eq!(t2.get_duration().quarters(), 4);
    assert_eq!(t3.get_duration().quarters(), 1);
}

#[test]
fn timerow_test() {
    let mut g = ChartTimeRow::new();

    assert!(g.count() == 0);
    assert!(!g.is_set(23));
    g.set(23);
    assert!(g.count() == 1);
    assert!(g.is_set(23));
    assert!(!g.is_set(22));
    assert!(!g.is_set(24));

    assert!(!g.is_set(0));
    g.set(0);
    assert!(g.count() == 2);
    assert!(g.is_set(0));
    assert!(!g.is_set(1));

    g.set_range(0..50);
    assert!(g.count() == 50);
    assert!(g.is_set(1));
    assert!(g.is_set(0));
    assert!(g.is_set(49));
    assert!(!g.is_set(50));

    // Successful smear
    let mut g2 = ChartTimeRow::new();
    match g.smear_transfer_to(&mut g2, 10, 0..50) {
        (Some(_), 10, 0) => assert!(true),
        _ => assert!(false),
    }
    assert!(g.count() == 40);
    assert!(g2.count() == 10);

    // Smear that required multiple passes
    g2.set_range(25..50);
    assert_eq!(g2.count(), 30);
    match g.smear_transfer_to(&mut g2, 10, 0..50) {
        (Some(_), 10, 0) => assert!(true),
        _ => assert!(false),
    }
    assert_eq!(g.count(), 30);
    assert_eq!(g2.count(), 40);

    // Failure to smear
    match g.smear_transfer_to(&mut g2, 11, 0..50) {
        (Some(_), 10, 1) => assert!(true),
        _ => assert!(false),
    }

    // Successful fill
    g2 = ChartTimeRow::new();
    g.set_range(0..50);
    match g.fill_transfer_to(&mut g2, 10, 5..20) {
        (Some(14), 10, 0) => assert!(true),
        _ => assert!(false),
    }

    // Another successful fill
    match g.fill_transfer_to(&mut g2, 2, 4..20) {
        (Some(15), 2, 0) => assert!(true),
        _ => assert!(false),
    }

    // Failure to allocate
    match g.fill_transfer_to(&mut g2, 2, 4..17) {
        (Some(16), 1, 1) => assert!(true),
        _ => assert!(false),
    }

    // Successful fill
    g2 = ChartTimeRow::new();
    g.set_range(0..50);
    match g.reverse_fill_transfer_to(&mut g2, 10, 25..40) {
        Ok(last) => assert!(last == 30),
        Err(_) => assert!(false),
    }

    // Another successful fill
    match g.reverse_fill_transfer_to(&mut g2, 2, 4..41) {
        Ok(last) => assert!(last == 29),
        Err(_) => assert!(false),
    }

    // Failure to allocate
    match g.reverse_fill_transfer_to(&mut g2, 2, 28..41) {
        Ok(_) => assert!(false),
        Err(unallocated) => assert!(unallocated == 1),
    }

    // Test row display
    assert_eq!(g2.get_weekly_summary(3), "    12  1".to_string());
    assert_eq!(g.get_weekly_summary(3), " 20  8  9".to_string());

}

#[test]
fn file_test() {

    // Set up a file
    let mut f = ConfigLines::new();
    f.add_line(Line::new_node_line(1, 0, "fred"));
    f.add_line(Line::new_attribute_line("plan", "10"));
    f.add_line(Line::new_attribute_line("budget", "10"));
    f.add_line(Line::new_node_line(5, 2, "child"));
    f.add_line(Line::new_attribute_line("plan", "5"));

    // Check peek/get on the first couple of lines
    assert_eq!(f.peek_line(), Some(Line::new_node_line(1, 0, "fred")));
    assert_eq!(f.peek_line(), Some(Line::new_node_line(1, 0, "fred")));
    assert_eq!(f.get_line(), Some(Line::new_node_line(1, 0, "fred")));
    assert_eq!(f.peek_line(), Some(Line::new_attribute_line("plan", "10")));

    // Read the rest of the lines, then check EOF handling
    f.get_line().unwrap();
    f.get_line().unwrap();
    f.get_line().unwrap();
    f.get_line().unwrap();
    assert_eq!(f.peek_line(), None);
    assert_eq!(f.get_line(), None);
}

#[test]
fn nodes_test() {
    let rc_root = Rc::new(RefCell::new(ConfigNode::new("root", 0, 0, 0)));

    // Set up config file
    let mut f = ConfigLines::new();
    f.add_line(Line::new_node_line(1, 1, "fred"));
    f.add_line(Line::new_attribute_line("plan", "10"));
    f.add_line(Line::new_attribute_line("budget", "10"));

    f.add_line(Line::new_node_line(5, 3, "child"));
    f.add_line(Line::new_attribute_line("plan", "5"));
    f.add_line(Line::new_attribute_line("default-plan", "3"));
    f.add_line(Line::new_attribute_line("resource", "smearprorata"));

    f.add_line(Line::new_node_line(7, 4, "grandchild"));
    f.add_line(Line::new_attribute_line("who", "rsl"));
    f.add_line(Line::new_attribute_line("schedule", "parallel"));
    f.add_line(Line::new_attribute_line("resource", "smearremaining"));
    f.add_line(Line::new_attribute_line("earliest-start", "2.3"));
    f.add_line(Line::new_attribute_line("latest-end", "2.4.3"));
    f.add_line(Line::new_attribute_line("non-managed", "true"));

    f.add_line(Line::new_node_line(8, 6, "greatgrandchild"));
    f.add_line(Line::new_attribute_line("C2.3.3", "1.5"));
    f.add_line(Line::new_attribute_line("C4", "1.25"));

    f.add_line(Line::new_node_line(9, 1, "sibling"));
    f.add_line(Line::new_attribute_line("plan", "5"));
    f.add_line(Line::new_attribute_line("Confusion", "Confusion"));

    f.add_line(Line::new_node_line(13, 3, "rsl"));

    f.add_line(Line::new_node_line(15, 1, "sibling2"));
    f.add_line(Line::new_node_line(17, 2, "s2child"));
    f.add_line(Line::new_attribute_line("default-plan", "52pcy"));
    f.add_line(Line::new_node_line(18, 3, "s2grandchild"));
    f.add_line(Line::new_attribute_line("schedule", "serial"));
    f.add_line(Line::new_node_line(20, 1, "sibling3"));
    f.add_line(Line::new_attribute_line("plan", "5, 2:10, 3:20"));

    f.add_line(Line::new_node_line(21, 1, "[chart]"));
    f.add_line(Line::new_attribute_line("weeks", "10"));

    f.add_line(Line::new_node_line(21, 1, "[people]"));
    f.add_line(Line::new_attribute_line("rf", "1.."));
    f.add_line(Line::new_attribute_line("rsl", "2..9.3"));

    // Restrict the scope of the mutable borrow of rc_root.
    {
        // And read the config file
        let mut root = rc_root.borrow_mut();
        root.consume_config(Some(&rc_root), &mut f).unwrap();

        // Test that the file is empty
        assert_eq!(f.peek_line(), None);

        // Test that the structure is as expected
        assert_eq!(root.count_nodes(), 13);
        assert_eq!(root.count_children(), 6);
    }

    // Test inheritance
    let root_ref = rc_root.borrow();
    let fred = root_ref.get_node_at_line(1)
        .unwrap()
        .upgrade()
        .unwrap();
    let child = root_ref.get_node_at_line(5)
        .unwrap()
        .upgrade()
        .unwrap();
    let grandchild = root_ref.get_node_at_line(7)
        .unwrap()
        .upgrade()
        .unwrap();
    let greatgrandchild = root_ref.get_node_at_line(8)
        .unwrap()
        .upgrade()
        .unwrap();
    let sibling = root_ref.get_node_at_line(9)
        .unwrap()
        .upgrade()
        .unwrap();
    let rsl = root_ref.get_node_at_line(13)
        .unwrap()
        .upgrade()
        .unwrap();
    let sibling2 = root_ref.get_node_at_line(15)
        .unwrap()
        .upgrade()
        .unwrap();
    //let sibling2child = root_ref.get_node_at_line(17).unwrap().upgrade().unwrap();
    let sibling2grandchild = root_ref.get_node_at_line(18)
        .unwrap()
        .upgrade()
        .unwrap();
    let sibling3 = root_ref.get_node_at_line(20)
        .unwrap()
        .upgrade()
        .unwrap();
    //let chart = root_ref.get_node_at_line(21).unwrap().upgrade().unwrap();

    assert_eq!(greatgrandchild.borrow().count_children(), 0);
    assert_eq!(grandchild.borrow()
                   .get_inherited_attribute::<String>("plan")
                   .unwrap()
                   .unwrap(),
               "5");
    assert_eq!(grandchild.borrow()
                   .get_inherited_attribute::<String>("budget")
                   .unwrap()
                   .unwrap(),
               "10");
    assert_eq!(grandchild.borrow()
                   .get_inherited_attribute::<String>("who")
                   .unwrap()
                   .unwrap(),
               "rsl");
    assert_eq!(grandchild.borrow().get_inherited_attribute::<String>("who2").unwrap(),
               None);

    // Test budget retrieval on nodes that do, and don't, include one,
    assert_eq!(grandchild.borrow().get_budget(), None);
    assert_eq!(fred.borrow().get_budget(), Some(Duration::new_days(10f32)));

    // Test "who" retrieval on nodes where:
    // - Local value is available
    // - Inherited value ia available
    // - No value is available
    // - Local name will serve as value
    let valid_who = vec!["rsl".to_string(), "rf".to_string()];
    assert_eq!(grandchild.borrow()
                   .get_who(&valid_who)
                   .unwrap()
                   .unwrap(),
               "rsl");
    assert_eq!(greatgrandchild.borrow()
                   .get_who(&valid_who)
                   .unwrap()
                   .unwrap(),
               "rsl");
    assert_eq!(child.borrow().get_who(&valid_who).unwrap(), None);
    assert_eq!(rsl.borrow()
                   .get_who(&valid_who)
                   .unwrap()
                   .unwrap(),
               "rsl");

    // Test "plan" retrieval on nodes where:
    // - Local value is available
    // - Inherited default-plan ia available (leaf&)
    // - Inherited default-plan ia available (non-leaf)
    // - No value is available
    // - Handling of pcm value on default-plan
    // - Handling of initial plan value
    // - Handling of post plan value
    // - Handling of intermediate plan values
    let two_weeks = Duration::new_days(10.0f32);
    let when = ChartTime::new("1").unwrap();
    assert_eq!(sibling.borrow()
                   .get_plan(&when, &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               20);
    assert_eq!(greatgrandchild.borrow()
                   .get_plan(&when, &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               12);
    assert_eq!(grandchild.borrow().get_plan(&when, &two_weeks).unwrap(),
               None);
    assert_eq!(sibling2.borrow().get_plan(&when, &two_weeks).unwrap(), None);
    assert_eq!(sibling2grandchild.borrow()
                   .get_plan(&when, &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               8);
    assert_eq!(sibling2grandchild.borrow()
                   .get_plan(&when, &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               8);

    assert_eq!(sibling3.borrow()
                   .get_plan(&ChartTime::new("1").unwrap(), &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               20);
    assert_eq!(sibling3.borrow()
                   .get_plan(&ChartTime::new("2").unwrap(), &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               40);
    assert_eq!(sibling3.borrow()
                   .get_plan(&ChartTime::new("2.3").unwrap(), &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               40);
    assert_eq!(sibling3.borrow()
                   .get_plan(&ChartTime::new("3").unwrap(), &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               80);
    assert_eq!(sibling3.borrow()
                   .get_plan(&ChartTime::new("4").unwrap(), &two_weeks)
                   .unwrap()
                   .unwrap()
                   .quarters(),
               80);

    // Test "schedule" retrieval on nodes where:
    // - Local value is serial
    // - Local value is parallel
    // - Local value is not present
    assert_eq!(sibling2grandchild.borrow().get_scheduling_strategy(),
               Ok(SchedulingStrategy::Serial));
    assert_eq!(grandchild.borrow().get_scheduling_strategy(),
               Ok(SchedulingStrategy::Parallel));
    assert_eq!(greatgrandchild.borrow().get_scheduling_strategy(),
               Ok(SchedulingStrategy::Parallel));

    // Test "resource" retrieval on nodes where:
    // - No value is available
    // - Local value is set
    // - Value is inherited
    assert_eq!(sibling2grandchild.borrow().get_resourcing_strategy().unwrap(),
               None);
    assert_eq!(child.borrow()
                   .get_resourcing_strategy()
                   .unwrap()
                   .unwrap(),
               ResourcingStrategy::SmearProRata);
    assert_eq!(greatgrandchild.borrow()
                   .get_resourcing_strategy()
                   .unwrap()
                   .unwrap(),
               ResourcingStrategy::SmearRemaining);

    // Test commitment retrieval on nodes where:
    // - There are no commitments
    // - There are no commitments, but there is an attribute starting with C.
    // - There are commitments
    assert_eq!(child.borrow().get_commitments().len(), 0);
    assert_eq!(sibling.borrow().get_commitments().len(), 0);
    assert_eq!(greatgrandchild.borrow().get_commitments()[0].0.get_quarter(),
               30);
    assert_eq!(greatgrandchild.borrow().get_commitments()[0].1.quarters(),
               6);
    assert_eq!(greatgrandchild.borrow().get_commitments()[1].0.get_quarter(),
               60);
    assert_eq!(greatgrandchild.borrow().get_commitments()[1].1.quarters(),
               5);

    // Test earliest-start retrieval on nodes where:
    // - There is a local value
    // - There is an inherited value
    // - There is no value
    assert_eq!(grandchild.borrow()
                   .get_earliest_start()
                   .unwrap()
                   .unwrap()
                   .get_quarter(),
               28);
    assert_eq!(greatgrandchild.borrow()
                   .get_earliest_start()
                   .unwrap()
                   .unwrap()
                   .get_quarter(),
               28);
    assert_eq!(child.borrow().get_earliest_start().unwrap(), None);

    // Test latest-end retrieval on nodes where:
    // - There is a local value
    // - There is an inherited value
    // - There is no value
    assert_eq!(grandchild.borrow()
                   .get_latest_end()
                   .unwrap()
                   .unwrap()
                   .get_quarter(),
               34);
    assert_eq!(greatgrandchild.borrow()
                   .get_latest_end()
                   .unwrap()
                   .unwrap()
                   .get_quarter(),
               34);
    assert_eq!(child.borrow().get_latest_end().unwrap(), None);

    // Test non-managed status retrieval on nodes where:
    // - There is a local value
    // - There is an inherited !value
    // - There is no value
    // Test non-managed status retrieval on nodes whersl:
    assert_eq!(grandchild.borrow().get_non_managed().unwrap(), true);
    assert_eq!(greatgrandchild.borrow().get_non_managed().unwrap(), true);
    assert_eq!(child.borrow().get_non_managed().unwrap(), false);

    // Test find_child_with_name where:
    // - The node exists
    // - The node does not exist
    let s2byname = root_ref.find_child_with_name("sibling2")
        .unwrap()
        .upgrade()
        .unwrap();
    assert_eq!(s2byname.borrow().count_children(), 1);
    match root_ref.find_child_with_name("siblingx") {
        None => {}
        Some(_) => {
            assert!(false);
        }
    }

    // Test global config retrieval
    let h: HashMap<String, String> = root_ref.get_global_config();
    assert_eq!(h["weeks"], "10".to_string());

    // Test people retrieval
    let h2 = root_ref.get_people(10).unwrap();
    assert!(h2.contains_key("rf"));
    assert!(h2.contains_key("rsl"));
    assert_eq!(h2.len(), 2);
    assert_eq!(h2.get("rf").unwrap().count(), 200);
    assert!(h2.get("rf").unwrap().is_set(0));
    assert!(h2.get("rf").unwrap().is_set(199));
    assert!(!h2.get("rf").unwrap().is_set(200));
    assert_eq!(h2.get("rsl").unwrap().count(), 152);
    assert!(!h2.get("rsl").unwrap().is_set(19));
    assert!(h2.get("rsl").unwrap().is_set(20));
    assert!(h2.get("rsl").unwrap().is_set(171));
    assert!(!h2.get("rsl").unwrap().is_set(172));
}
