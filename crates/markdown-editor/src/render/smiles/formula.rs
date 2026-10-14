use std::collections::BTreeMap;

use omgkit_core::MolBuilder;
use omgkit_depict::style::Style;

use super::Drawing;

/// Count a sanitized molecule, including bracket and implicit hydrogens.
/// Disconnected components retain separate formulas and charges.
fn molecular_formula(mol: &MolBuilder) -> Option<String> {
    let mut seen = vec![false; mol.num_atoms()];
    let mut components = Vec::new();
    for start in 0..mol.num_atoms() {
        if seen[start] {
            continue;
        }
        let mut counts = BTreeMap::<(&str, u16), usize>::new();
        let mut charge = 0i32;
        let mut pending = vec![start as u32];
        seen[start] = true;
        while let Some(ix) = pending.pop() {
            let atom = &mol.atoms()[ix as usize];
            if atom.atomic_num == 0 {
                return None; // A wildcard has no definite molecular formula.
            }
            let element = omgkit_core::element::by_atomic_num(atom.atomic_num)?;
            *counts.entry((element.symbol, atom.isotope)).or_default() += 1;
            let hydrogens = usize::from(atom.num_explicit_hs) + usize::from(atom.num_implicit_hs);
            if hydrogens > 0 {
                *counts.entry(("H", 0)).or_default() += hydrogens;
            }
            charge += i32::from(atom.formal_charge);
            for (neighbor, _) in mol.neighbors(ix) {
                if !seen[neighbor as usize] {
                    seen[neighbor as usize] = true;
                    pending.push(neighbor);
                }
            }
        }
        // Hill order: C, H, then alphabetical; without carbon, all alphabetical.
        let has_carbon = counts.keys().any(|(symbol, _)| *symbol == "C");
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort_by_key(|((symbol, isotope), _)| {
            let rank = match (*symbol, has_carbon) {
                ("C", true) => 0,
                ("H", true) => 1,
                _ => 2,
            };
            (rank, *symbol, *isotope)
        });
        let mut formula = String::new();
        for ((symbol, isotope), count) in counts {
            if isotope != 0 {
                formula.push_str(&digits(isotope.into(), true));
            }
            formula.push_str(symbol);
            if count > 1 {
                formula.push_str(&digits(count, false));
            }
        }
        if charge != 0 {
            if charge.unsigned_abs() > 1 {
                formula.push_str(&digits(charge.unsigned_abs() as usize, true));
            }
            formula.push(if charge > 0 { '⁺' } else { '⁻' });
        }
        components.push(formula);
    }
    (!components.is_empty()).then(|| components.join(" · "))
}

fn digits(n: usize, superscript: bool) -> String {
    const SUB: [char; 10] = ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
    const SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let glyphs = if superscript { SUP } else { SUB };
    n.to_string()
        .bytes()
        .map(|d| glyphs[(d - b'0') as usize])
        .collect()
}

#[derive(Default)]
struct GlyphPath(String);

impl ttf_parser::OutlineBuilder for GlyphPath {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push_str(&format!("M{x} {y}"));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push_str(&format!("L{x} {y}"));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push_str(&format!("Q{x1} {y1} {x} {y}"));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.push_str(&format!("C{x1} {y1} {x2} {y2} {x} {y}"));
    }
    fn close(&mut self) {
        self.0.push('Z');
    }
}

pub(super) fn annotate(drawing: &mut Drawing, mol: &MolBuilder, style: &Style) {
    let Some(formula) = molecular_formula(mol) else {
        return;
    };
    // Measure the same face the depiction's labels are drawn with.
    let Some(font) = super::formula_font() else {
        return;
    };
    let Ok(face) = ttf_parser::Face::parse(font, 0) else {
        return;
    };
    let Some(advances) = formula
        .chars()
        .map(|c| face.glyph_hor_advance(face.glyph_index(c)?))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let fs = style.atom_label_pt;
    let scale = fs / f64::from(face.units_per_em());
    let text_width = advances.iter().map(|a| f64::from(*a) * scale).sum::<f64>();
    let ascent = f64::from(face.ascender()) * scale;
    let descent = -f64::from(face.descender()) * scale;
    let gap = fs * 0.8;
    let width = drawing.width.max(text_width + fs * 2.);
    let height = drawing.height + gap + ascent + descent;
    // Embed the bundled upright glyphs. Native SVG and browser font fallback
    // can otherwise select different faces for Unicode sub/superscripts.
    let mut paths = String::new();
    let mut advance = 0u32;
    for (c, width) in formula.chars().zip(advances) {
        let mut outline = GlyphPath::default();
        let glyph = face.glyph_index(c).expect("formula glyph was measured");
        if face.outline_glyph(glyph, &mut outline).is_some() {
            paths.push_str(&format!(
                "<path transform=\"translate({advance} 0)\" d=\"{}\"/>",
                outline.0
            ));
        } else if !c.is_whitespace() {
            return;
        }
        advance += u32::from(width);
    }
    drawing.svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.2}\" height=\"{height:.2}\" viewBox=\"0 0 {width:.2} {height:.2}\"><rect width=\"{width:.2}\" height=\"{height:.2}\" fill=\"#fff\"/><g transform=\"translate({x:.2} 0)\">{svg}</g><g class=\"molecular-formula\" aria-label=\"{formula}\" fill=\"#000\"><title>{formula}</title><g transform=\"translate({label_x:.2} {baseline:.2}) scale({scale:.6} {negative_scale:.6})\">{paths}</g></g></svg>",
        x = (width - drawing.width) / 2.,
        svg = drawing.svg,
        label_x = (width - text_width) / 2.,
        negative_scale = -scale,
        baseline = drawing.height + gap + ascent,
    );
    drawing.width = width;
    drawing.height = height;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn molecular_formulas_count_hydrogens_isotopes_charges_and_fragments() {
        for (smiles, expected) in [
            ("CCO", "C₂H₆O"),
            ("C(=O)O", "CH₂O₂"),
            ("CO", "CH₄O"),
            ("COC=O", "C₂H₄O₂"),
            ("CC(=O)Oc1ccccc1C(=O)O", "C₉H₈O₄"),
            ("c1cc[nH]c1", "C₄H₅N"),
            ("[NH4+]", "H₄N⁺"),
            ("[Ca+2]", "Ca²⁺"),
            ("[Na+].[Cl-]", "Na⁺ · Cl⁻"),
            ("[13CH4]", "¹³CH₄"),
            ("[2H]O[2H]", "²H₂O"),
            ("[H]O[H]", "H₂O"),
            ("[C]", "C"),
        ] {
            let mut mol = omgkit_io::smiles::parse(smiles).unwrap();
            omgkit_chem::pipeline::sanitize(&mut mol).unwrap();
            assert_eq!(
                molecular_formula(&mol).as_deref(),
                Some(expected),
                "{smiles}"
            );
        }
        let mol = omgkit_io::smiles::parse("*CO").unwrap();
        assert!(molecular_formula(&mol).is_none());
    }
}
