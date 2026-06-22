use std::collections::HashMap;
use anyhow::{Result, anyhow, Context};
use zbus::zvariant::{OwnedValue, Value};

#[derive(Debug, Clone, serde::Serialize)]
pub struct MenuNode {
    pub id: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggle_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub toggle_state: Option<i32>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Self>,
}

impl MenuNode {
    pub fn new(id: i32, props: HashMap<String, OwnedValue>, children: Vec<OwnedValue>) -> Result<Self> {
        macro_rules! get {
            ($name:literal, $type:ty, $err:literal) => {
                match props.get($name) {
                    Some(x) => Some(
                        TryInto::<$type>::try_into(
                            x.downcast_ref::<Value>()
                             .with_context(|| anyhow!($err))?
                        )?
                    ),
                    None => None,
                }
            };
        }

        let children = children.into_iter()
            .map(TryInto::<(i32, HashMap<String, OwnedValue>, Vec<OwnedValue>)>::try_into)
            .collect::<Result<Vec<_>, _>>()?;

        let children = children.into_iter()
            .map(|(id, props, children)| Self::new(id, props, children))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            id,
            label: get!("label", String, "label is not a string")
                // remove underscore markers
                .map(|x| x.replace("_", "")),
            enabled: get!("enabled", bool, "enabled is not a bool"),
            visible: get!("visible", bool, "visible is not a bool"),
            toggle_type: get!("toggle-type", String, "toggle_type is not a string"),
            toggle_state: get!("toggle-state", i32, "enabled is not a bool"),
            children,
        })
    }

    #[allow(unused)]
    pub fn is_root(&self) -> bool {
        self.id == 0
    }

    #[allow(unused)]
    pub fn is_separator(&self) -> bool {
        self.label.is_none()
            && self.enabled.is_none()
            && self.label.is_none()
            && self.toggle_type.is_none()
            && self.toggle_state.is_none()
            && self.children.is_empty()
    }

    /// Filter out hidden or disabled nodes
    pub fn filter_nodes(root: &mut MenuNode, hidden: bool, disabled: bool) {
        for id in root.children.iter().map(|x| x.id).collect::<Vec<_>>() {
            let (visible, enabled, index) = {
                let index = root.children.iter().position(|x| x.id == id).unwrap();
                (root.children[index].visible.clone(), root.children[index].enabled.clone(), index)
            };

            // is it hidden
            match (hidden, visible) {
                (false, Some(false)) => {
                    root.children.swap_remove(index);
                    continue
                },
                _ => {}
            };

            // is it disabled
            match (disabled, enabled) {
                (false, Some(false)) => {
                    root.children.swap_remove(index);
                    continue
                },
                _ => {}
            };

            // it is visible so now recursively go into the children
            for child in &mut root.children[index].children {
                Self::filter_nodes(child, hidden, disabled);
            }
        }
    }

}

