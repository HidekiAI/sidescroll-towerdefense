use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{ValidationMessage, ValidationResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityClass {
    Stationary(StationarySubClass),
    Mobile(MobileSubClass),
    Organic(OrganicSubClass),
    Composite(CompositeSubClass),
}

impl EntityClass {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "tower" => Some(EntityClass::Stationary(StationarySubClass::Tower)),
            "trap" => Some(EntityClass::Stationary(StationarySubClass::Trap)),
            "structure" => Some(EntityClass::Stationary(StationarySubClass::Structure)),
            "vehicle" => Some(EntityClass::Mobile(MobileSubClass::Vehicle)),
            "beast" => Some(EntityClass::Mobile(MobileSubClass::Beast)),
            "projectile" => Some(EntityClass::Mobile(MobileSubClass::Projectile)),
            "hero" => Some(EntityClass::Organic(OrganicSubClass::Hero)),
            "adventurer" => Some(EntityClass::Organic(OrganicSubClass::Adventurer)),
            "soldier" => Some(EntityClass::Organic(OrganicSubClass::Soldier)),
            "enemy" => Some(EntityClass::Organic(OrganicSubClass::Enemy)),
            "convoy" => Some(EntityClass::Composite(CompositeSubClass::Convoy)),
            "wave" => Some(EntityClass::Composite(CompositeSubClass::Wave)),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            EntityClass::Stationary(StationarySubClass::Tower) => "tower",
            EntityClass::Stationary(StationarySubClass::Trap) => "trap",
            EntityClass::Stationary(StationarySubClass::Structure) => "structure",
            EntityClass::Mobile(MobileSubClass::Vehicle) => "vehicle",
            EntityClass::Mobile(MobileSubClass::Beast) => "beast",
            EntityClass::Mobile(MobileSubClass::Projectile) => "projectile",
            EntityClass::Organic(OrganicSubClass::Hero) => "hero",
            EntityClass::Organic(OrganicSubClass::Adventurer) => "adventurer",
            EntityClass::Organic(OrganicSubClass::Soldier) => "soldier",
            EntityClass::Organic(OrganicSubClass::Enemy) => "enemy",
            EntityClass::Composite(CompositeSubClass::Convoy) => "convoy",
            EntityClass::Composite(CompositeSubClass::Wave) => "wave",
        }
    }
}

/// serde bridge so `EntityClass` serializes as its flat lowercase string
/// (`"tower"`, `"structure"`, …) instead of serde's nested-enum representation.
mod entity_class_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::EntityClass;

    pub fn serialize<S: Serializer>(class: &EntityClass, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(class.as_str())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<EntityClass, D::Error> {
        let s = String::deserialize(d)?;
        EntityClass::from_str(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown entity class: \"{s}\"")))
    }
}

/// Elemental affinity of an entity def. Exact, case-sensitive values; unknown
/// values fail deserialization (no fallback).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Physical,
    Fire,
    Ice,
    Lightning,
    Holy,
    Dark,
}

impl Element {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "physical" => Some(Element::Physical),
            "fire" => Some(Element::Fire),
            "ice" => Some(Element::Ice),
            "lightning" => Some(Element::Lightning),
            "holy" => Some(Element::Holy),
            "dark" => Some(Element::Dark),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Element::Physical => "physical",
            Element::Fire => "fire",
            Element::Ice => "ice",
            Element::Lightning => "lightning",
            Element::Holy => "holy",
            Element::Dark => "dark",
        }
    }
}

impl EntityClass {
    pub fn allowed_modifier_slots(self) -> &'static [(&'static str, i32)] {
        match self {
            EntityClass::Stationary(StationarySubClass::Tower) => &[
                ("defense", 100),
                ("speed", 200),
                ("status", 300),
                ("element", 400),
            ],
            EntityClass::Stationary(StationarySubClass::Trap) => {
                &[("damage", 100), ("radius", 200), ("status", 300)]
            }
            EntityClass::Stationary(StationarySubClass::Structure) => &[],
            EntityClass::Mobile(MobileSubClass::Vehicle) => {
                &[("speed", 100), ("defense", 200), ("element", 300)]
            }
            EntityClass::Mobile(MobileSubClass::Beast) => &[
                ("speed", 100),
                ("defense", 200),
                ("status", 300),
                ("element", 400),
            ],
            EntityClass::Mobile(MobileSubClass::Projectile) => &[],
            EntityClass::Organic(OrganicSubClass::Hero) => &[("all", 0)],
            EntityClass::Organic(OrganicSubClass::Adventurer) => &[("all", 0)],
            EntityClass::Organic(OrganicSubClass::Soldier) => &[("defense", 100), ("speed", 200)],
            EntityClass::Organic(OrganicSubClass::Enemy) => {
                &[("buff", 100), ("debuff", 200), ("status", 300)]
            }
            EntityClass::Composite(_) => &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StationarySubClass {
    Tower,
    Trap,
    Structure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobileSubClass {
    Vehicle,
    Beast,
    Projectile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganicSubClass {
    Hero,
    Adventurer,
    Soldier,
    Enemy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositeSubClass {
    Convoy,
    Wave,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityDef {
    pub id: i64,
    pub entity_key: String,
    pub entity_class_id: i64,
    pub config_domain_id: Option<i64>,
    pub script_name: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EntityDefEditor {
    pub key: String,
    #[serde(with = "entity_class_serde")]
    #[schemars(with = "String")]
    pub class: EntityClass,
    pub width_tiles: f64,
    pub height_tiles: f64,
    pub max_hp: i32,
    pub speed_pps: i32,
    pub attack_range_tiles: i32,
    pub attack_power: i32,
    pub element: Element,
    pub action_cooldown_ticks: i32,
    pub projectile_type: String,
    pub requires_ground: bool,
    pub requires_ceiling: bool,
    #[serde(default)]
    pub weight: f64,
    #[serde(default)]
    pub max_weight: f64,
}

/// Prototype-based entity override: every field except `key` is optional.
/// A world zip may contain `entity_defs.json` with full `EntityDefEditor`
/// entries.  When the key matches a framework prototype, unspecified fields
/// inherit from the prototype.  When the key is new, all fields must be
/// provided (enforced by validation, not serde — `Option` defaults to `None`).
///
/// Schema verification: `#[serde(deny_unknown_fields)]` rejects typos,
/// `Option<T>` types enforce exact type matches, and `EntityClass`/`Element`
/// enums reject unknown values at parse time.
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct EntityDefOverride {
    pub key: String,
    #[serde(default)]
    #[serde(with = "opt_entity_class_serde")]
    #[schemars(with = "Option<String>")]
    pub class: Option<EntityClass>,
    #[serde(default)]
    pub width_tiles: Option<f64>,
    #[serde(default)]
    pub height_tiles: Option<f64>,
    #[serde(default)]
    pub max_hp: Option<i32>,
    #[serde(default)]
    pub speed_pps: Option<i32>,
    #[serde(default)]
    pub attack_range_tiles: Option<i32>,
    #[serde(default)]
    pub attack_power: Option<i32>,
    #[serde(default)]
    pub element: Option<Element>,
    #[serde(default)]
    pub action_cooldown_ticks: Option<i32>,
    #[serde(default)]
    pub projectile_type: Option<String>,
    #[serde(default)]
    pub requires_ground: Option<bool>,
    #[serde(default)]
    pub requires_ceiling: Option<bool>,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub max_weight: Option<f64>,
}

impl EntityDefOverride {
    /// Merge this override onto a framework prototype, returning a new
    /// `EntityDefEditor` where specified fields override the prototype and
    /// unspecified fields keep their prototype values.
    pub fn merge(&self, prototype: &EntityDefEditor) -> EntityDefEditor {
        EntityDefEditor {
            key: self.key.clone(),
            class: self.class.unwrap_or(prototype.class),
            width_tiles: self.width_tiles.unwrap_or(prototype.width_tiles),
            height_tiles: self.height_tiles.unwrap_or(prototype.height_tiles),
            max_hp: self.max_hp.unwrap_or(prototype.max_hp),
            speed_pps: self.speed_pps.unwrap_or(prototype.speed_pps),
            attack_range_tiles: self
                .attack_range_tiles
                .unwrap_or(prototype.attack_range_tiles),
            attack_power: self.attack_power.unwrap_or(prototype.attack_power),
            element: self.element.unwrap_or(prototype.element),
            action_cooldown_ticks: self
                .action_cooldown_ticks
                .unwrap_or(prototype.action_cooldown_ticks),
            projectile_type: self
                .projectile_type
                .clone()
                .unwrap_or_else(|| prototype.projectile_type.clone()),
            requires_ground: self.requires_ground.unwrap_or(prototype.requires_ground),
            requires_ceiling: self.requires_ceiling.unwrap_or(prototype.requires_ceiling),
            weight: self.weight.unwrap_or(prototype.weight),
            max_weight: self.max_weight.unwrap_or(prototype.max_weight),
        }
    }

    /// Promote a complete override (where all fields are `Some`) into a
    /// full `EntityDefEditor`.  Returns `None` if any field is `None`.
    pub fn into_entity_def(self) -> Option<EntityDefEditor> {
        Some(EntityDefEditor {
            key: self.key,
            class: self.class?,
            width_tiles: self.width_tiles?,
            height_tiles: self.height_tiles?,
            max_hp: self.max_hp?,
            speed_pps: self.speed_pps?,
            attack_range_tiles: self.attack_range_tiles?,
            attack_power: self.attack_power?,
            element: self.element?,
            action_cooldown_ticks: self.action_cooldown_ticks?,
            projectile_type: self.projectile_type?,
            requires_ground: self.requires_ground?,
            requires_ceiling: self.requires_ceiling?,
            weight: self.weight.unwrap_or(0.0),
            max_weight: self.max_weight.unwrap_or(0.0),
        })
    }
}

/// serde bridge for `Option<EntityClass>`: serializes/deserializes as an
/// optional flat lowercase string (`"tower"`, `"vehicle"`, …).
mod opt_entity_class_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    use super::EntityClass;

    pub fn serialize<S: Serializer>(class: &Option<EntityClass>, s: S) -> Result<S::Ok, S::Error> {
        match class {
            Some(c) => s.serialize_some(c.as_str()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<EntityClass>, D::Error> {
        let opt: Option<String> = Option::deserialize(d)?;
        match opt {
            Some(s) => EntityClass::from_str(&s)
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom(format!("unknown entity class: \"{s}\""))),
            None => Ok(None),
        }
    }
}

impl EntityDefEditor {
    pub fn resolve_class(&self) -> Option<EntityClass> {
        Some(self.class)
    }

    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::valid();

        if self.key.is_empty() {
            result = result.with_message(
                ValidationMessage::error("ENTITY_EMPTY_KEY", "Entity key must not be empty")
                    .with_field("key"),
            );
        }

        if self.width_tiles <= 0.0 {
            result = result.with_message(
                ValidationMessage::error("ENTITY_INVALID_WIDTH", "Width must be positive")
                    .with_field("width_tiles")
                    .with_value(&self.width_tiles.to_string()),
            );
        }

        if self.height_tiles <= 0.0 {
            result = result.with_message(
                ValidationMessage::error("ENTITY_INVALID_HEIGHT", "Height must be positive")
                    .with_field("height_tiles")
                    .with_value(&self.height_tiles.to_string()),
            );
        }

        if self.max_hp <= 0 {
            result = result.with_message(
                ValidationMessage::error("ENTITY_INVALID_HP", "Max HP must be positive")
                    .with_field("max_hp")
                    .with_value(&self.max_hp.to_string()),
            );
        }

        result
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityInstance {
    pub id: i64,
    pub instance_key: String,
    pub entity_def_id: i64,
    pub world_tile_x: Option<i32>,
    pub world_tile_y: Option<i32>,
    pub flip_y: bool,
    pub state_id: i64,
    pub is_active: bool,
    pub owner_id: i64,
    pub spawned_at: String,
    pub expires_at: Option<String>,
    pub captured_at: Option<String>,
    pub captured_from_id: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModifierOperation {
    Add,
    Multiply,
    Replace,
}

impl ModifierOperation {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "add" => Some(ModifierOperation::Add),
            "multiply" => Some(ModifierOperation::Multiply),
            "replace" => Some(ModifierOperation::Replace),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModifierDef {
    pub id: i64,
    pub modifier_key: String,
    pub category_id: i64,
    pub operation_id: i64,
    pub base_value: f64,
    pub priority: i32,
    pub max_stacks: i32,
    pub duration_sec: Option<i32>,
    pub description_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModifierGroup {
    pub modifier_def_id: i64,
    pub operation: ModifierOperation,
    pub base_value: f64,
    pub effective_stacks: i32,
    pub priority: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttributeDef {
    pub id: i64,
    pub attribute_key: String,
    pub value_type_id: i64,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub unit: Option<String>,
    pub description_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstanceModifier {
    pub id: i64,
    pub instance_id: i64,
    pub modifier_def_id: i64,
    pub source_id: i64,
    pub is_active: bool,
    pub applied_at: String,
    pub expires_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstanceStatus {
    pub instance_id: i64,
    pub current_hp: Option<i32>,
    pub current_mp: Option<i32>,
    pub current_stamina: Option<i32>,
    pub last_action_at: Option<String>,
    pub kill_count: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityRuntimeState {
    pub needs_attention: bool,
    pub next_turn: u64,
    pub needs_refresh: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityLimits {
    pub max_entities_per_world: i32,
    pub max_entities_on_screen: i32,
    pub max_process_per_frame: i32,
    pub max_off_screen_process: i32,
}

impl Default for EntityLimits {
    fn default() -> Self {
        Self {
            max_entities_per_world: 500,
            max_entities_on_screen: 50,
            max_process_per_frame: 30,
            max_off_screen_process: 10,
        }
    }
}

pub fn resolve_attribute(
    base: f64,
    override_multiplier: Option<f64>,
    override_addend: Option<f64>,
    modifiers: &[ModifierGroup],
) -> f64 {
    let after_overrides = match (override_multiplier, override_addend) {
        (Some(mul), Some(add)) => base * mul + add,
        (Some(mul), None) => base * mul,
        (None, Some(add)) => base + add,
        (None, None) => base,
    };

    modifiers.iter().fold(after_overrides, |acc, group| {
        let effective = group.base_value * group.effective_stacks as f64;
        match group.operation {
            ModifierOperation::Add => acc + effective,
            ModifierOperation::Multiply => acc * effective,
            ModifierOperation::Replace => effective,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tower_prototype() -> EntityDefEditor {
        EntityDefEditor {
            key: "arrow_tower".into(),
            class: EntityClass::from_str("tower").unwrap(),
            width_tiles: 1.0,
            height_tiles: 2.5,
            max_hp: 500,
            speed_pps: 0,
            attack_range_tiles: 5,
            attack_power: 100,
            element: Element::Physical,
            action_cooldown_ticks: 60,
            projectile_type: "arrow".into(),
            requires_ground: true,
            requires_ceiling: false,
            weight: 0.0,
            max_weight: 0.0,
        }
    }

    #[test]
    fn entity_override_merge_inherits_unspecified_fields() {
        let proto = tower_prototype();
        // Override only max_hp + attack_power; everything else must inherit.
        let ov = EntityDefOverride {
            key: "arrow_tower".into(),
            width_tiles: None,
            height_tiles: None,
            class: None,
            max_hp: Some(900),
            speed_pps: None,
            attack_range_tiles: None,
            attack_power: Some(250),
            element: None,
            action_cooldown_ticks: None,
            projectile_type: None,
            requires_ground: None,
            requires_ceiling: None,
            weight: None,
            max_weight: None,
        };
        let merged = ov.merge(&proto);
        assert_eq!(merged.max_hp, 900);
        assert_eq!(merged.attack_power, 250);
        // Unspecified fields inherit from the prototype.
        assert_eq!(merged.width_tiles, 1.0);
        assert_eq!(merged.height_tiles, 2.5);
        assert_eq!(merged.attack_range_tiles, 5);
        assert_eq!(merged.projectile_type, "arrow");
        assert_eq!(
            merged.class,
            EntityClass::Stationary(StationarySubClass::Tower)
        );
        assert!(merged.requires_ground);
        assert!(!merged.requires_ceiling);
    }

    #[test]
    fn entity_override_full_into_entity_def() {
        let ov = EntityDefOverride {
            key: "new_unit".into(),
            class: Some(EntityClass::Stationary(StationarySubClass::Trap)),
            width_tiles: Some(1.0),
            height_tiles: Some(0.5),
            max_hp: Some(200),
            speed_pps: Some(0),
            attack_range_tiles: Some(2),
            attack_power: Some(150),
            element: Some(Element::Fire),
            action_cooldown_ticks: Some(30),
            projectile_type: Some(String::new()),
            requires_ground: Some(true),
            requires_ceiling: Some(false),
            weight: Some(1.0),
            max_weight: Some(2.0),
        };
        let full = ov.into_entity_def().expect("complete override promotes");
        assert_eq!(full.key, "new_unit");
        assert_eq!(full.max_hp, 200);
        assert_eq!(full.element, Element::Fire);
    }

    #[test]
    fn entity_override_incomplete_into_entity_def_is_none() {
        let ov = EntityDefOverride {
            key: "partial".into(),
            class: Some(EntityClass::from_str("tower").unwrap()),
            width_tiles: None,
            height_tiles: None,
            max_hp: None,
            speed_pps: None,
            attack_range_tiles: None,
            attack_power: None,
            element: None,
            action_cooldown_ticks: None,
            projectile_type: None,
            requires_ground: None,
            requires_ceiling: None,
            weight: None,
            max_weight: None,
        };
        assert!(ov.into_entity_def().is_none());
    }
}
