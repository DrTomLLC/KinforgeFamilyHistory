use kinforge_core::{models::*, KinforgeResult};
use kinforge_storage::Database;

/// Render a simple ASCII family tree centered on a person.
pub fn ascii_family_tree(
    db: &Database,
    person_id: &PersonId,
    depth: u32,
) -> KinforgeResult<String> {
    let mut out = String::new();
    render_node(db, person_id, depth, 0, &mut out)?;
    Ok(out)
}

fn render_node(
    db: &Database,
    person_id: &PersonId,
    max_depth: u32,
    current_depth: u32,
    out: &mut String,
) -> KinforgeResult<()> {
    let person = db.get_person(person_id)?;
    let indent = "    ".repeat(current_depth as usize);
    let connector = if current_depth == 0 { "" } else { "└── " };
    out.push_str(&format!(
        "{}{}{}\n",
        indent,
        connector,
        person.display_name()
    ));

    if current_depth < max_depth {
        let rels = db.list_relationships_for_person(person_id)?;
        let children: Vec<&Relationship> = rels
            .iter()
            .filter(|r| r.rel_type == RelationshipType::ParentChild && r.person1_id == *person_id)
            .collect();

        for child_rel in children {
            render_node(db, &child_rel.person2_id, max_depth, current_depth + 1, out)?;
        }
    }
    Ok(())
}
