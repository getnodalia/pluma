//! Attribute uniqueness checks missing from the pinned shared XML reader's strict mode.
//!
//! Temporary consumer guard: parsing remains in nodalia-xml. Move this invariant into the
//! shared strict reader and remove this second event pass when that dependency is updated.

/// Check names before canonical/local-name rewriting can collapse distinct attributes.
/// The event reader and namespace scope use the same caller-supplied resource caps.
pub(super) fn validate(xml: &[u8], limits: nodalia_xml::Limits) -> Result<(), String> {
    use std::collections::BTreeSet;

    let opts = nodalia_xml::Options { limits, doctype: nodalia_xml::Doctype::Reject, ..Default::default() }.strict();
    let mut events = opts.events(xml);
    let mut bindings: Vec<(String, String)> = Vec::new();
    let mut scopes = Vec::new();
    while let Some(event) = events.next().map_err(|e| e.to_string())? {
        match event {
            nodalia_xml::Event::Start { attrs, .. } => {
                scopes.push(bindings.len());
                let mut raw = BTreeSet::new();
                for (key, value) in &attrs {
                    if !raw.insert(key.as_str()) {
                        return Err(format!("duplicate attribute `{key}`"));
                    }
                    if let Some(prefix) = key.strip_prefix("xmlns:") {
                        if bindings.len() >= limits.max_attrs {
                            return Err("too many in-scope namespace declarations".into());
                        }
                        bindings.push((prefix.to_string(), value.clone()));
                    }
                }
                let mut expanded = BTreeSet::new();
                for (key, _) in &attrs {
                    if key == "xmlns" || key.starts_with("xmlns:") {
                        continue;
                    }
                    // Default namespaces do not apply to attributes. Preserve undeclared
                    // prefixes for standalone SVG library fragments, as the importer did.
                    let name = match key.split_once(':') {
                        Some(("xml", local)) => ("http://www.w3.org/XML/1998/namespace", local),
                        Some((prefix, local)) => {
                            bindings.iter().rev().find(|(p, _)| p == prefix).map_or(("", key.as_str()), |(_, uri)| (uri.as_str(), local))
                        }
                        None => ("", key.as_str()),
                    };
                    if !expanded.insert(name) {
                        return Err(format!("duplicate expanded attribute `{key}`"));
                    }
                }
            }
            nodalia_xml::Event::End { .. } => bindings.truncate(scopes.pop().unwrap_or(0)),
            _ => {}
        }
    }
    Ok(())
}
