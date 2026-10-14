use gpui_kit::{Size, size};
use omgkit_core::{AtomData, AtomFlags, BondData, BondDirection, BondFlags, BondOrder, MolBuilder};
use omgkit_depict::{generate, render::scene, style::Style, svg::to_svg};
use omgkit_io::smarts::{self, AtomExpr, AtomPrim, BondExpr, BondPrim, QueryMol};

mod formula;

/// Bytes of the face molecular-formula labels are measured and drawn with:
/// the system Noto Sans (the depiction style's first family), falling back
/// to any sans. Loaded once; `None` disables formula annotation only.
pub(super) fn formula_font() -> Option<&'static [u8]> {
    static FONT: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    FONT.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        let id = db.query(&usvg::fontdb::Query {
            families: &[
                usvg::fontdb::Family::Name("Noto Sans"),
                usvg::fontdb::Family::SansSerif,
            ],
            ..Default::default()
        })?;
        db.with_face_data(id, |data, _| data.to_vec())
    })
    .as_deref()
}

struct Drawing {
    svg: String,
    width: f64,
    height: f64,
    body_height: f64,
}

#[cfg(test)]
pub(super) fn render(code: &str, background: u32, foreground: u32) -> Option<(String, Size<f32>)> {
    render_with_formulas(code, background, foreground, false)
}

pub(super) fn render_with_formulas(
    code: &str,
    background: u32,
    foreground: u32,
    molecular_formulas: bool,
) -> Option<(String, Size<f32>)> {
    if code.len() > 65_536 {
        return None;
    }
    let source = code.trim();
    // A fence holds one complete molecule or reaction, never a .smi record with a name or
    // several lines silently reduced to their first molecule.
    if source.is_empty() || source.chars().any(char::is_whitespace) {
        return None;
    }
    let reaction = smarts::parse_reaction(source).ok();
    let mut molecules = if let Some(reaction) = &reaction {
        if reaction.reactants.is_empty()
            || reaction.products.is_empty()
            || !reaction.duplicate_map_numbers().is_empty()
            || source
                .as_bytes()
                .windows(2)
                .any(|w| matches!(w, b".." | b".>" | b">."))
            || source.starts_with('.')
            || source.ends_with('.')
        {
            return None;
        }
        let queries: Vec<_> = reaction
            .reactants
            .iter()
            .chain(&reaction.products)
            .chain(&reaction.agents)
            .collect();
        if queries.iter().map(|q| q.num_atoms()).sum::<usize>() > 512
            || queries.iter().map(|q| q.num_bonds()).sum::<usize>() > 1024
        {
            return None;
        }
        queries
            .into_iter()
            .map(query_molecule)
            .collect::<Option<Vec<_>>>()?
    } else {
        vec![omgkit_io::smiles::parse(source).ok()?]
    };
    // Apply the budget to the entire reaction before doing any layout work.
    if molecules.iter().any(|mol| mol.num_atoms() == 0)
        || molecules.iter().map(|mol| mol.num_atoms()).sum::<usize>() > 512
        || molecules.iter().map(|mol| mol.num_bonds()).sum::<usize>() > 1024
    {
        return None;
    }
    let style = Style {
        font_family: "Noto Sans, sans-serif",
        ..Style::CHEMDRAW_DEFAULT
    };
    let mut drawings = Vec::with_capacity(molecules.len());
    for mol in &mut molecules {
        omgkit_chem::pipeline::sanitize(mol).ok()?;
        // Sanitization alone does not perceive double-bond geometry. Skipping this
        // step can silently draw the opposite cis/trans configuration in release.
        omgkit_io::stereo::perceive_bond_stereo(mol);
        let depiction = generate(mol, &style);
        if !depiction.is_clean()
            || depiction
                .coords
                .iter()
                .any(|p| !p.x.is_finite() || !p.y.is_finite())
        {
            return None;
        }
        // scene() includes any explicit hydrogens added to depict stereocentres.
        let drawing = scene(mol, &depiction, &style);
        if !drawing.width.is_finite()
            || !drawing.height.is_finite()
            || drawing.width <= 0.
            || drawing.height <= 0.
        {
            return None;
        }
        let mut drawing = Drawing {
            svg: to_svg(&drawing, &style),
            width: drawing.width,
            height: drawing.height,
            body_height: drawing.height,
        };
        if molecular_formulas {
            formula::annotate(&mut drawing, mol, &style);
        }
        drawings.push(drawing);
    }
    let svg = if let Some(reaction) = reaction {
        reaction_svg(
            &drawings,
            reaction.reactants.len(),
            reaction.products.len(),
            &style,
        )
    } else {
        drawings.pop()?.svg
    };
    adapt_svg(&svg, background, foreground)
}

/// QueryMol::topology contains placeholder atoms and bonds. Materialize only
/// elemental templates whose annotations can be represented faithfully, never
/// canonicalize the placeholder graph or pick a branch of a logical query.
fn query_molecule(query: &QueryMol) -> Option<MolBuilder> {
    let mut mol = MolBuilder::with_capacity(query.num_atoms(), query.num_bonds());
    for expr in &query.atoms {
        let mut atom = AtomData::default();
        let mut seen = 0u8;
        query_atom(expr, &mut atom, &mut seen)?;
        if seen & 1 == 0 {
            return None;
        }
        mol.add_atom_data(atom);
    }
    for (topology, expr) in query.topology.bonds().iter().zip(&query.bonds) {
        let mut bond = BondData::new(topology.begin, topology.end, BondOrder::Single);
        if *expr == BondExpr::default_bond() {
            if [topology.begin, topology.end]
                .iter()
                .all(|a| mol.atoms()[*a as usize].flags.contains(AtomFlags::AROMATIC))
            {
                bond.order = BondOrder::Aromatic;
            }
        } else if let BondExpr::Prim(prim) = expr {
            match prim {
                BondPrim::Single => {}
                BondPrim::Double => bond.order = BondOrder::Double,
                BondPrim::Triple => bond.order = BondOrder::Triple,
                BondPrim::Quadruple => bond.order = BondOrder::Quadruple,
                BondPrim::Aromatic => bond.order = BondOrder::Aromatic,
                BondPrim::UpRight => bond.direction = BondDirection::UpRight,
                BondPrim::DownRight => bond.direction = BondDirection::DownRight,
                BondPrim::Dative => bond.order = BondOrder::Dative,
                BondPrim::DativeReversed => {
                    bond.order = BondOrder::Dative;
                    std::mem::swap(&mut bond.begin, &mut bond.end);
                }
                _ => return None,
            }
        } else {
            return None;
        }
        // The chemistry pipeline requires both the aromatic order and flag,
        // just as the library's SMILES parser sets them.
        if bond.order == BondOrder::Aromatic {
            bond.flags.insert(BondFlags::AROMATIC);
        }
        mol.add_bond_data(bond).ok()?;
    }
    // SMARTS H counts include neighboring explicit hydrogen atoms, whereas
    // MolBuilder's num_explicit_hs counts only bracket hydrogen annotations.
    for ix in 0..mol.num_atoms() {
        if mol.atoms()[ix].flags.contains(AtomFlags::NO_IMPLICIT) {
            let neighboring_hs = mol
                .neighbors(ix as u32)
                .filter(|(a, _)| mol.atoms()[*a as usize].atomic_num == 1)
                .count();
            let atom = mol.atom_mut(ix as u32)?;
            atom.num_explicit_hs = atom
                .num_explicit_hs
                .checked_sub(u8::try_from(neighboring_hs).ok()?)?;
        }
    }
    omgkit_chem::pipeline::sanitize(&mut mol).ok()?;
    omgkit_io::stereo::perceive_bond_stereo(&mut mol);
    // The library owns canonical traversal and stereochemical serialization.
    // This temporary representation never replaces the document's source.
    let canonical = omgkit_io::canon::canonical_smiles(&mol).smiles;
    omgkit_io::smiles::parse(&canonical).ok()
}

fn query_atom(expr: &AtomExpr, atom: &mut AtomData, seen: &mut u8) -> Option<()> {
    if let AtomExpr::And(parts) = expr {
        for part in parts {
            query_atom(part, atom, seen)?;
        }
        return Some(());
    }
    let AtomExpr::Prim(prim) = expr else {
        return None;
    };
    let field = match prim {
        AtomPrim::Element { z, aromatic } => {
            atom.atomic_num = *z;
            if *aromatic == Some(true) {
                atom.flags.insert(AtomFlags::AROMATIC);
            }
            1
        }
        AtomPrim::Any => {
            atom.atomic_num = 0;
            1
        }
        AtomPrim::TotalHs(n) => {
            atom.num_explicit_hs = u8::try_from(*n).ok()?;
            atom.flags.insert(AtomFlags::NO_IMPLICIT);
            2
        }
        AtomPrim::Charge(n) => {
            atom.formal_charge = i8::try_from(*n).ok()?;
            4
        }
        AtomPrim::Isotope(n) => {
            atom.isotope = *n;
            8
        }
        AtomPrim::AtomMap(n) => {
            atom.atom_map = *n;
            16
        }
        AtomPrim::Chirality(tag) => {
            atom.chiral_tag = *tag;
            32
        }
        _ => return None,
    };
    if *seen & field != 0 {
        return None;
    }
    *seen |= field;
    Some(())
}

/// Compose parsed reactants, products and agents at a common bond scale.
fn reaction_svg(drawings: &[Drawing], reactants: usize, products: usize, style: &Style) -> String {
    let (left, rest) = drawings.split_at(reactants);
    let (right, agents) = rest.split_at(products);
    let gap = style.bond_length_pt;
    let row_width = |row: &[Drawing]| {
        row.iter().map(|d| d.width).sum::<f64>() + gap * row.len().saturating_sub(1) as f64
    };
    let row_height = |row: &[Drawing]| row.iter().map(|d| d.body_height).fold(0., f64::max);
    let caption_height = |row: &[Drawing]| {
        row.iter()
            .map(|d| d.height - d.body_height)
            .fold(0., f64::max)
    };
    let agent_width = row_width(agents);
    let agent_height = row_height(agents) + caption_height(agents);
    let arrow = (gap * 2.).max(agent_width);
    let arrow_start = row_width(left) + gap;
    let arrow_end = arrow_start + arrow;
    let width = arrow_end + gap + row_width(right);
    let main_height = row_height(left)
        .max(row_height(right))
        .max(style.atom_label_pt * 2.);
    let mid = (main_height / 2.).max(agent_height + gap / 2.);
    let height = mid + main_height / 2. + caption_height(left).max(caption_height(right));
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.2}\" height=\"{height:.2}\" viewBox=\"0 0 {width:.2} {height:.2}\"><rect width=\"{width:.2}\" height=\"{height:.2}\" fill=\"#fff\"/>"
    );
    append_row(&mut svg, left, "reactant", 0., mid, style);
    append_row(&mut svg, right, "product", arrow_end + gap, mid, style);
    append_row(
        &mut svg,
        agents,
        "agent",
        arrow_start + (arrow - agent_width) / 2.,
        row_height(agents) / 2.,
        style,
    );
    let tip = style.atom_label_pt / 2.;
    svg.push_str(&format!(
        "<path class=\"reaction-arrow\" d=\"M{arrow_start:.2},{mid:.2} H{arrow_end:.2} M{back:.2},{top:.2} L{arrow_end:.2},{mid:.2} L{back:.2},{bottom:.2}\" fill=\"none\" stroke=\"#000\" stroke-width=\"{stroke}\"/>",
        back=arrow_end-tip, top=mid-tip, bottom=mid+tip, stroke=style.line_width_pt,
    ));
    svg.push_str("</svg>");
    svg
}

fn append_row(
    svg: &mut String,
    drawings: &[Drawing],
    role: &str,
    mut x: f64,
    mid: f64,
    style: &Style,
) {
    for (ix, drawing) in drawings.iter().enumerate() {
        if ix > 0 {
            let center = x + style.bond_length_pt / 2.;
            let half = style.atom_label_pt / 2.;
            svg.push_str(&format!(
                "<path class=\"reaction-plus\" d=\"M{left:.2},{mid:.2} H{right:.2} M{center:.2},{top:.2} V{bottom:.2}\" fill=\"none\" stroke=\"#000\" stroke-width=\"{stroke}\"/>",
                left=center-half, right=center+half, top=mid-half, bottom=mid+half, stroke=style.line_width_pt,
            ));
            x += style.bond_length_pt;
        }
        svg.push_str(&format!(
            "<g class=\"reaction-{role}\" transform=\"translate({x:.2} {y:.2})\">{svg}</g>",
            y = mid - drawing.body_height / 2.,
            svg = drawing.svg,
        ));
        x += drawing.width;
    }
}

fn adapt_svg(svg: &str, background: u32, foreground: u32) -> Option<(String, Size<f32>)> {
    if svg.len() > 4 * 1024 * 1024 {
        return None;
    }
    let doc = usvg::roxmltree::Document::parse(svg).ok()?;
    let root = doc.root_element();
    let dimension = |name| {
        root.attribute(name)?
            .parse::<f32>()
            .ok()
            .filter(|n| n.is_finite() && *n > 0.)
    };
    let (width, height) = (dimension("width")?, dimension("height")?);
    let scale = (1600. / width).min(1600. / height).min(1.);
    let size = size(width * scale, height * scale);
    let mut edits = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        for attr in node.attributes() {
            let replacement = match (attr.name(), attr.value()) {
                ("width", _) if node == root => Some(size.width.to_string()),
                ("height", _) if node == root => Some(size.height.to_string()),
                // The pinned 2D backend writes monochrome ink and an opaque
                // white canvas. Change attributes only, preserving viewBox,
                // label text, bond geometry, and explicit fill="none".
                ("fill" | "stroke", "#000") => Some(format!("#{foreground:06x}")),
                ("fill", "#fff")
                    if node.has_tag_name("rect")
                        && node.parent().is_some_and(|p| p.has_tag_name("svg")) =>
                {
                    Some(format!("#{background:06x}"))
                }
                _ => None,
            };
            if let Some(value) = replacement {
                edits.push((attr.range_value(), value));
            }
        }
    }
    let mut svg = svg.to_string();
    for (range, value) in edits.into_iter().rev() {
        svg.replace_range(range, &value);
    }
    Some((svg, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg_tree(svg: &str) -> usvg::Tree {
        let mut options = usvg::Options::default();
        options.fontdb_mut().load_system_fonts();
        usvg::Tree::from_str(svg, &options).expect("molecule SVG must parse")
    }

    #[test]
    fn smiles_structures_draw_visible_ink_in_both_themes() {
        for source in [
            "CCO",
            "c1ccccc1",
            "CC(=O)Oc1ccccc1C(=O)O",
            "[Na+].[Cl-]",
            "N[C@@H](C)C(=O)O",
            "C/C=C/C",
            "C/C=C\\C",
            "[13CH4]",
            "O",
        ] {
            for (background, foreground) in [(0xffffff, 0x222222), (0x171717, 0xeeeeee)] {
                let (svg, size) = render(source, background, foreground)
                    .unwrap_or_else(|| panic!("failed to render {source}"));
                let tree = svg_tree(&svg);
                assert!(size.width > 0. && size.width <= 1600.);
                assert!(size.height > 0. && size.height <= 1600.);
                let mut pixels = resvg::tiny_skia::Pixmap::new(
                    size.width.ceil() as u32,
                    size.height.ceil() as u32,
                )
                .unwrap();
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::identity(),
                    &mut pixels.as_mut(),
                );
                let bg = [
                    (background >> 16) as u8,
                    (background >> 8) as u8,
                    background as u8,
                    255,
                ];
                assert!(
                    pixels
                        .data()
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .filter(|p| { p[3] == 255 && **p != bg })
                        .count()
                        > 10,
                    "no molecular ink for {source}"
                );
                assert!(svg.contains(&format!("#{foreground:06x}")));
                assert!(svg.contains(&format!("#{background:06x}")));
                assert!(!svg.contains("\"#000\"") && !svg.contains("\"#fff\""));
            }
        }
        assert_eq!(
            render(" \nCCO\r\n", 0xffffff, 0),
            render("CCO", 0xffffff, 0)
        );
    }

    #[test]
    fn smiles_stereoisomers_have_distinct_bond_geometry() {
        let draw = |source| render(source, 0xffffff, 0).unwrap().0;
        let first = draw("N[C@@H](C)C(=O)O");
        let second = draw("N[C@H](C)C(=O)O");
        assert_ne!(first, second);
        // One enantiomer has a solid wedge, the other a hashed wedge.
        assert_ne!(first.contains("<path"), second.contains("<path"));
        assert_ne!(draw("C/C=C/C"), draw("C/C=C\\C"));
    }

    #[test]
    fn molecular_formulas_label_each_reaction_component_in_both_themes() {
        let source = "[C:1](=[O:2])[OH:3].[OH:4][C:5]>O>[C:1](=[O:2])[O:4][C:5]";
        for (background, foreground) in [(0xffffff, 0x222222), (0x171717, 0xeeeeee)] {
            let (plain, plain_size) = render(source, background, foreground).unwrap();
            let (svg, size) = render_with_formulas(source, background, foreground, true).unwrap();
            assert!(!plain.contains("molecular-formula"));
            assert!(size.height > plain_size.height);
            let doc = usvg::roxmltree::Document::parse(&svg).unwrap();
            let labels: Vec<_> = doc
                .descendants()
                .filter(|n| n.attribute("class") == Some("molecular-formula"))
                .map(|n| n.attribute("aria-label").unwrap())
                .collect();
            assert_eq!(labels, ["CH₂O₂", "CH₄O", "C₂H₄O₂", "H₂O"]);
            assert!(!svg.contains("\"#fff\"") && !svg.contains("\"#000\""));
            let tree = svg_tree(&svg);
            let mut pixels =
                resvg::tiny_skia::Pixmap::new(size.width.ceil() as u32, size.height.ceil() as u32)
                    .unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut pixels.as_mut(),
            );
            assert!(
                pixels
                    .pixels()
                    .iter()
                    .any(|p| p.alpha() == 255 && p.red() == (foreground >> 16) as u8)
            );
        }
        let (svg, _) = render_with_formulas("*CO", 0xffffff, 0, true).unwrap();
        assert!(!svg.contains("molecular-formula"));
    }

    #[test]
    fn smiles_reactions_draw_all_components_and_the_arrow_in_both_themes() {
        let source = "[C:1](=[O:2])[OH:3].[OH:4][C:5]>>[C:1](=[O:2])[O:4][C:5]";
        for (background, foreground) in [(0xffffff, 0x222222), (0x171717, 0xeeeeee)] {
            let (svg, size) = render(source, background, foreground).expect("mapped reaction");
            let doc = usvg::roxmltree::Document::parse(&svg).unwrap();
            let count = |class| {
                doc.descendants()
                    .filter(|n| n.attribute("class") == Some(class))
                    .count()
            };
            assert_eq!(count("reaction-reactant"), 2);
            assert_eq!(count("reaction-product"), 1);
            assert_eq!(count("reaction-plus"), 1);
            assert_eq!(count("reaction-arrow"), 1);
            assert!(!svg.contains("\"#fff\"") && !svg.contains("\"#000\""));
            let tree = svg_tree(&svg);
            let mut pixels =
                resvg::tiny_skia::Pixmap::new(size.width.ceil() as u32, size.height.ceil() as u32)
                    .unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut pixels.as_mut(),
            );
            // Each component and the arrow must contain visible ink, not just an SVG canvas.
            for node in doc.descendants().filter(|n| {
                matches!(
                    n.attribute("class"),
                    Some("reaction-reactant" | "reaction-product")
                )
            }) {
                let translation = node
                    .attribute("transform")
                    .unwrap()
                    .trim_start_matches("translate(")
                    .trim_end_matches(')');
                let origin: Vec<f32> = translation
                    .split_whitespace()
                    .map(|v| v.parse().unwrap())
                    .collect();
                let child = node.children().find(|n| n.has_tag_name("svg")).unwrap();
                let width: f32 = child.attribute("width").unwrap().parse().unwrap();
                let height: f32 = child.attribute("height").unwrap().parse().unwrap();
                let bg = (background >> 16) as u8;
                assert!(
                    (origin[0].ceil() as u32..(origin[0] + width).floor() as u32).any(|x| {
                        (origin[1].ceil() as u32..(origin[1] + height).floor() as u32).any(|y| {
                            pixels
                                .pixel(x, y)
                                .is_some_and(|p| p.alpha() == 255 && p.red() != bg)
                        })
                    })
                );
            }
        }
        let svg = render("CCO.O>>CC=O.O", 0xffffff, 0).unwrap().0;
        assert_eq!(svg.matches("class=\"reaction-plus\"").count(), 2);
        let svg = render("N->[Cu]>>N.[Cu]", 0xffffff, 0).unwrap().0;
        assert_eq!(svg.matches("class=\"reaction-reactant\"").count(), 1);
        assert_eq!(svg.matches("class=\"reaction-product\"").count(), 2);
        assert!(render("c1cc[nH]c1>>c1cc[nH]c1", 0xffffff, 0).is_some());
    }

    #[test]
    fn smiles_reactions_remain_source_if_incomplete_invalid_or_over_budget() {
        for source in [
            "CCO>>",
            ">>CCO",
            "CCO>>>CCN",
            "CCO>>CCN>>CCO",
            "CCO.>>CCN",
            "CCO>>.CCN",
            "CCO>>C1CC",
            "CCO>>C1CC2CCC1CC2",
            "[C:1].[C:1]>>[C:1]",
            "[C,N:1]>>[C:1]",
            "[C;D3:1]>>[C:1]",
            "[$(C.C):1]>>[C:1]",
            "C~O>>CO",
            "CCO>[N,O]>CC=O",
        ] {
            assert!(render(source, 0xffffff, 0).is_none(), "accepted {source}");
        }
        let source = format!("{}>>{}", "C".repeat(257), "C".repeat(256));
        assert!(render(&source, 0xffffff, 0).is_none());
    }

    #[test]
    fn smiles_reaction_templates_preserve_elements_maps_hydrogens_and_stereo() {
        let canonical = |mut mol: MolBuilder| {
            omgkit_chem::pipeline::sanitize(&mut mol).unwrap();
            omgkit_io::stereo::perceive_bond_stereo(&mut mol);
            omgkit_io::canon::canonical_smiles(&mol).smiles
        };
        for (template, molecule) in [
            ("[C:1](=[O:2])[OH:3]", "[CH:1](=[O:2])[OH:3]"),
            ("[OH:4][C:5]", "[OH:4][CH3:5]"),
            ("c1ccccc1", "c1ccccc1"),
            ("c1cc[nH]c1", "c1cc[nH]c1"),
            ("[13CH3:1][NH3+:2]", "[13CH3:1][NH3+:2]"),
            ("[H][OH1:2]", "[H][O:2]"),
            ("N[C@@H:1](C)C(=O)O", "N[C@@H:1](C)C(=O)O"),
            ("N[C@H:1](C)C(=O)O", "N[C@H:1](C)C(=O)O"),
            ("F[C@]1(Cl)CCNC1", "F[C@]1(Cl)CCNC1"),
            ("C/C=C/C", "C/C=C/C"),
            ("C/C=C\\C", "C/C=C\\C"),
            ("[N:1]->[Cu:2]", "[NH3:1]->[Cu:2]"),
            ("*", "*"),
        ] {
            let query = smarts::parse(template).unwrap();
            let mol = query_molecule(&query).unwrap_or_else(|| panic!("{template}"));
            assert_eq!(
                canonical(mol),
                canonical(omgkit_io::smiles::parse(molecule).unwrap()),
                "template {template} must retain chemical properties",
            );
        }
    }

    #[test]
    fn smiles_reaction_groups_and_agents_keep_their_layout_roles() {
        let svg = render("(CCO.O)>O.[Na+]>CC=O.O", 0xffffff, 0).unwrap().0;
        let doc = usvg::roxmltree::Document::parse(&svg).unwrap();
        let nodes = |class| {
            doc.descendants()
                .filter(move |n| n.attribute("class") == Some(class))
        };
        assert_eq!(nodes("reaction-reactant").count(), 1);
        assert_eq!(nodes("reaction-product").count(), 2);
        assert_eq!(nodes("reaction-agent").count(), 2);
        assert_eq!(nodes("reaction-plus").count(), 2);
        let arrow = nodes("reaction-arrow").next().unwrap();
        let start = arrow
            .attribute("d")
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap();
        let (_, mid) = start.trim_start_matches('M').split_once(',').unwrap();
        let mid: f64 = mid.parse().unwrap();
        for agent in nodes("reaction-agent") {
            let transform = agent.attribute("transform").unwrap();
            let (_, y) = transform.trim_end_matches(')').split_once(' ').unwrap();
            let y: f64 = y.parse().unwrap();
            let svg = agent.children().find(|n| n.has_tag_name("svg")).unwrap();
            let height: f64 = svg.attribute("height").unwrap().parse().unwrap();
            assert!(
                y >= 0. && y + height < mid,
                "agents must sit above the arrow"
            );
        }
        let tree = svg_tree(&svg);
        assert!(tree.size().width() > tree.size().height());
    }

    #[test]
    fn smiles_invalid_oversized_and_degraded_structures_remain_source() {
        for source in [
            "",
            " \n",
            "not smiles",
            "C1CC",
            "C(C)(C)(C)(C)C",
            "CCO ethanol",
            "CCO\nCCN",
            "CCO\tCCN",
            "C1CC2CCC1CC2",
            "C1/C=C/CCCCC1",
        ] {
            assert!(render(source, 0xffffff, 0).is_none(), "accepted {source:?}");
        }
        assert!(render(&"C".repeat(513), 0xffffff, 0).is_none());
        assert!(render(&"C".repeat(65_537), 0xffffff, 0).is_none());
    }

    #[test]
    fn smiles_svg_viewport_is_bounded_without_changing_bond_coordinates() {
        let (svg, size) = adapt_svg(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"6400\" height=\"3200\" viewBox=\"0 0 6400 3200\"><rect width=\"6400\" height=\"3200\" fill=\"#fff\"/><path d=\"M0 0L6400 3200\" stroke=\"#000\" fill=\"none\"/></svg>",
            0x171717, 0xeeeeee,
        ).unwrap();
        assert_eq!(size, gpui_kit::size(1600., 800.));
        let tree = svg_tree(&svg);
        assert_eq!((tree.size().width(), tree.size().height()), (1600., 800.));
        assert!(svg.contains("viewBox=\"0 0 6400 3200\""));
        assert!(svg.contains("d=\"M0 0L6400 3200\""));
        assert!(svg.contains("fill=\"none\""));
        for dim in ["0", "-1", "NaN", "inf"] {
            assert!(adapt_svg(&format!("<svg width=\"{dim}\" height=\"10\"/>"), 0, 0).is_none());
        }
        assert!(adapt_svg("invalid SVG", 0, 0).is_none());
    }
}
