use crate::*;
use minecraft_format::{
    effect::Effect,
    entity::{EntityMetadata, EntityMetadataValue},
    ids::entities::Entity as EntityType,
    packets::Direction,
    paintings::Painting,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc::Sender;

const EMPTY_EQUIPMENT: &EntityEquipment = &EntityEquipment::none();
// TODO once stable: const NO_ATTRIBUTES: &BTreeMap<String, (f64, Vec<minecraft_format::entity::EntityAttributeModifier>)> = &BTreeMap::new();

pub struct EntityEquipment {
    pub main_hand: Slot,
    pub off_hand: Slot,
    pub boots: Slot,
    pub leggings: Slot,
    pub chestplate: Slot,
    pub helmet: Slot,
}

impl EntityEquipment {
    const fn none() -> EntityEquipment {
        EntityEquipment {
            main_hand: Slot { item: None },
            off_hand: Slot { item: None },
            boots: Slot { item: None },
            leggings: Slot { item: None },
            chestplate: Slot { item: None },
            helmet: Slot { item: None },
        }
    }
}

pub enum Entity {
    LivingEntity {
        uuid: u128,
        entity_type: EntityType,
        x: f64,
        y: f64,
        z: f64,
        yaw: u8,
        pitch: u8,
        head_pitch: u8,
        velocity_x: f64,
        velocity_y: f64,
        velocity_z: f64,
        effects: Vec<(Effect, u8)>,
        equipment: EntityEquipment,
        attributes: BTreeMap<String, (f64, Vec<minecraft_format::entity::EntityAttributeModifier>)>,
        metadata: BTreeMap<u8, EntityMetadataValue>,
    },
    Player {
        uuid: u128,
        x: f64,
        y: f64,
        z: f64,
        yaw: u8,
        pitch: u8,
        velocity_x: f64,
        velocity_y: f64,
        velocity_z: f64,
        effects: Vec<(Effect, u8)>,
        equipment: EntityEquipment,
        attributes: BTreeMap<String, (f64, Vec<minecraft_format::entity::EntityAttributeModifier>)>,
        metadata: BTreeMap<u8, EntityMetadataValue>,
    },
    ExperienceOrb {
        count: u16,
        x: f64,
        y: f64,
        z: f64,
        velocity_x: f64,
        velocity_y: f64,
        velocity_z: f64,
        metadata: BTreeMap<u8, EntityMetadataValue>,
    },
    Painting {
        uuid: u128,
        motive: Painting,
        x: i32,
        y: i32,
        z: i32,
        direction: Direction,
        metadata: BTreeMap<u8, EntityMetadataValue>,
    },
    OtherEntity {
        uuid: u128,
        entity_type: EntityType,
        x: f64,
        y: f64,
        z: f64,
        yaw: u8,
        pitch: u8,
        data: i32,
        velocity_x: f64,
        velocity_y: f64,
        velocity_z: f64,
        effects: Vec<(Effect, u8)>,
        equipment: EntityEquipment,
        attributes: BTreeMap<String, (f64, Vec<minecraft_format::entity::EntityAttributeModifier>)>,
        metadata: BTreeMap<u8, EntityMetadataValue>,
    },
}

impl Entity {
    pub fn get_position(&self) -> (f64, f64, f64) {
        match self {
            Entity::LivingEntity { x, y, z, .. }
            | Entity::Player { x, y, z, .. }
            | Entity::ExperienceOrb { x, y, z, .. }
            | Entity::OtherEntity { x, y, z, .. } => (*x, *y, *z),
            Entity::Painting { x, y, z, .. } => (*x as f64, *y as f64, *z as f64),
        }
    }

    fn set_position(&mut self, new_x: f64, new_y: f64, new_z: f64) {
        match self {
            Entity::LivingEntity { x, y, z, .. }
            | Entity::Player { x, y, z, .. }
            | Entity::ExperienceOrb { x, y, z, .. }
            | Entity::OtherEntity { x, y, z, .. } => {
                *x = new_x;
                *y = new_y;
                *z = new_z;
            }
            Entity::Painting { x, y, z, .. } => {
                *x = new_x as i32;
                *y = new_y as i32;
                *z = new_z as i32;
            }
        }
    }

    fn set_rotation(&mut self, new_yaw: u8, new_pitch: u8) {
        match self {
            Entity::LivingEntity { yaw, pitch, .. } | Entity::Player { yaw, pitch, .. } | Entity::OtherEntity { yaw, pitch, .. } => {
                *yaw = new_yaw;
                *pitch = new_pitch;
            }
            Entity::ExperienceOrb { .. } | Entity::Painting { .. } => {
                warn!("Tried to set rotation for an experience orb or a painting");
            }
        }
    }

    fn set_velocity(&mut self, new_velocity_x: f64, new_velocity_y: f64, new_velocity_z: f64) {
        match self {
            Entity::LivingEntity {
                velocity_x,
                velocity_y,
                velocity_z,
                ..
            }
            | Entity::OtherEntity {
                velocity_x,
                velocity_y,
                velocity_z,
                ..
            }
            | Entity::Player {
                velocity_x,
                velocity_y,
                velocity_z,
                ..
            }
            | Entity::ExperienceOrb {
                velocity_x,
                velocity_y,
                velocity_z,
                ..
            } => {
                *velocity_x = new_velocity_x;
                *velocity_y = new_velocity_y;
                *velocity_z = new_velocity_z;
            }
            Entity::Painting { .. } => {
                warn!("Tried to set velocity for a painting");
            }
        }
    }

    fn remove_effect(&mut self, effect: Effect) {
        match self {
            Entity::LivingEntity { effects, .. } | Entity::OtherEntity { effects, .. } | Entity::Player { effects, .. } => {
                while let Some(idx) = effects.iter().position(|(e, _)| *e == effect) {
                    effects.remove(idx);
                }
            }
            Entity::ExperienceOrb { .. } | Entity::Painting { .. } => {
                warn!("Tried to remove an effect for an experience orb or a painting");
            }
        }
    }

    fn add_effect(&mut self, effect: Effect, level: u8) {
        match self {
            Entity::LivingEntity { effects, .. } | Entity::OtherEntity { effects, .. } | Entity::Player { effects, .. } => {
                effects.push((effect, level))
            }
            Entity::ExperienceOrb { .. } | Entity::Painting { .. } => {
                warn!("Tried to remove an effect for an experience orb or a painting");
            }
        }
    }

    pub fn get_equipment(&self) -> &EntityEquipment {
        match self {
            Entity::LivingEntity { equipment, .. } | Entity::OtherEntity { equipment, .. } | Entity::Player { equipment, .. } => equipment,
            Entity::ExperienceOrb { .. } | Entity::Painting { .. } => {
                warn!("Tried to get equipment for an experience orb or a painting");
                EMPTY_EQUIPMENT
            }
        }
    }

    fn get_mut_equipment(&mut self) -> Option<&mut EntityEquipment> {
        match self {
            Entity::LivingEntity { equipment, .. } | Entity::OtherEntity { equipment, .. } | Entity::Player { equipment, .. } => Some(equipment),
            Entity::ExperienceOrb { .. } | Entity::Painting { .. } => {
                warn!("Tried to set equipment for an experience orb or a painting");
                None
            }
        }
    }

    pub fn get_attributes(&self) -> Option<&BTreeMap<String, (f64, Vec<minecraft_format::entity::EntityAttributeModifier>)>> {
        match self {
            Entity::LivingEntity { attributes, .. } | Entity::OtherEntity { attributes, .. } | Entity::Player { attributes, .. } => Some(attributes),
            Entity::ExperienceOrb { .. } | Entity::Painting { .. } => {
                warn!("Tried to set equipment for an experience orb or a painting");
                None
            }
        }
    }

    fn get_mut_attributes(&mut self) -> Option<&mut BTreeMap<String, (f64, Vec<minecraft_format::entity::EntityAttributeModifier>)>> {
        match self {
            Entity::LivingEntity { attributes, .. } | Entity::OtherEntity { attributes, .. } | Entity::Player { attributes, .. } => Some(attributes),
            Entity::ExperienceOrb { .. } | Entity::Painting { .. } => {
                warn!("Tried to set equipment for an experience orb or a painting");
                None
            }
        }
    }

    pub fn get_metadata(&self) -> &BTreeMap<u8, EntityMetadataValue> {
        match self {
            Entity::ExperienceOrb { metadata, .. }
            | Entity::LivingEntity { metadata, .. }
            | Entity::OtherEntity { metadata, .. }
            | Entity::Painting { metadata, .. }
            | Entity::Player { metadata, .. } => metadata,
        }
    }
}

pub struct Entities {
    sender: Sender<Vec<u8>>,
    entities: HashMap<i32, Entity>,
}

impl Entities {
    pub fn new(sender: Sender<Vec<u8>>) -> Self {
        Entities {
            sender,
            entities: HashMap::new(),
        }
    }

    pub fn add_self(&mut self, entity_id: i32) {
        self.entities.insert(
            entity_id,
            Entity::Player {
                uuid: 0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                velocity_x: 0.0,
                velocity_y: 0.0,
                velocity_z: 0.0,
                yaw: 0,
                pitch: 0,
                effects: Vec::new(),
                equipment: EntityEquipment::none(),
                attributes: BTreeMap::new(),
                metadata: BTreeMap::new(),
            },
        );
    }

    pub fn handle_spawn_entity_packet(
        &mut self,
        id: i32,
        uuid: u128,
        entity_type: EntityType,
        x: f64,
        y: f64,
        z: f64,
        pitch: u8,
        yaw: u8,
        data: i32,
        velocity_x: i16,
        velocity_y: i16,
        velocity_z: i16,
    ) {
        self.entities.insert(
            id,
            Entity::OtherEntity {
                uuid,
                entity_type,
                x,
                y,
                z,
                yaw,
                pitch,
                data,
                velocity_x: velocity_x as f64 / 8000.0,
                velocity_y: velocity_y as f64 / 8000.0,
                velocity_z: velocity_z as f64 / 8000.0,
                effects: Vec::new(),
                equipment: EntityEquipment::none(),
                attributes: BTreeMap::new(),
                metadata: BTreeMap::new(),
            },
        );
    }

    pub fn handle_spawn_living_entity_packet(
        &mut self,
        id: i32,
        uuid: u128,
        entity_type: EntityType,
        x: f64,
        y: f64,
        z: f64,
        yaw: u8,
        pitch: u8,
        head_pitch: u8,
        velocity_x: i16,
        velocity_y: i16,
        velocity_z: i16,
    ) {
        self.entities.insert(
            id,
            Entity::LivingEntity {
                uuid,
                entity_type,
                x,
                y,
                z,
                yaw,
                pitch,
                head_pitch,
                velocity_x: velocity_x as f64 / 8000.0,
                velocity_y: velocity_y as f64 / 8000.0,
                velocity_z: velocity_z as f64 / 8000.0,
                effects: Vec::new(),
                equipment: EntityEquipment::none(),
                attributes: BTreeMap::new(),
                metadata: BTreeMap::new(),
            },
        );
    }

    pub fn handle_spawn_experience_orb_packet(&mut self, id: i32, x: f64, y: f64, z: f64, count: i16) {
        self.entities.insert(
            id,
            Entity::ExperienceOrb {
                count: std::cmp::max(count, 0) as u16,
                x,
                y,
                z,
                velocity_x: 0.0,
                velocity_y: 0.0,
                velocity_z: 0.0,
                metadata: BTreeMap::new(),
            },
        );
    }

    pub fn handle_spawn_player_packet(&mut self, id: i32, uuid: u128, x: f64, y: f64, z: f64, yaw: u8, pitch: u8) {
        self.entities.insert(
            id,
            Entity::Player {
                uuid,
                x,
                y,
                z,
                velocity_x: 0.0,
                velocity_y: 0.0,
                velocity_z: 0.0,
                yaw,
                pitch,
                effects: Vec::new(),
                equipment: EntityEquipment::none(),
                attributes: BTreeMap::new(),
                metadata: BTreeMap::new(),
            },
        );
    }

    pub fn handle_spawn_painting_packet(&mut self, id: i32, uuid: u128, motive: Painting, location: Position, direction: Direction) {
        self.entities.insert(
            id,
            Entity::Painting {
                uuid,
                motive,
                x: location.x,
                y: location.y as i32,
                z: location.z,
                direction,
                metadata: BTreeMap::new(),
            },
        );
    }

    pub fn handle_entity_position_packet(&mut self, entity_id: i32, delta_x: i16, delta_y: i16, delta_z: i16, _on_ground: bool) {
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The moved entity does not exist ({})", entity_id);
                return;
            }
        };
        let (prev_x, prev_y, prev_z) = entity.get_position();
        let new_x = delta_x as f64 / (128.0 * 32.0) + prev_x;
        let new_y = delta_y as f64 / (128.0 * 32.0) + prev_y;
        let new_z = delta_z as f64 / (128.0 * 32.0) + prev_z;
        entity.set_position(new_x, new_y, new_z);
    }

    pub fn handle_entity_position_and_rotation_packet(
        &mut self,
        entity_id: i32,
        delta_x: i16,
        delta_y: i16,
        delta_z: i16,
        yaw: u8,
        pitch: u8,
        _on_ground: bool,
    ) {
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The moved entity does not exist (with rotation)({})", entity_id);
                return;
            }
        };
        let (prev_x, prev_y, prev_z) = entity.get_position();
        let new_x = delta_x as f64 / (128.0 * 32.0) + prev_x;
        let new_y = delta_y as f64 / (128.0 * 32.0) + prev_y;
        let new_z = delta_z as f64 / (128.0 * 32.0) + prev_z;
        entity.set_position(new_x, new_y, new_z);
        entity.set_rotation(yaw, pitch);
    }

    pub fn handle_entity_movement_packet(&mut self, entity_id: i32) {
        if !self.entities.contains_key(&entity_id) {
            warn!("Unsupported entity init by movement packet. Ignoring entity {}", entity_id);
        }
    }

    pub fn handle_destroy_entities_packet(&mut self, entity_ids: Vec<i32>) {
        for entity_id in entity_ids {
            if self.entities.remove(&entity_id).is_none() {
                warn!("Removed an entity that was already removed")
            }
        }
    }

    pub fn handle_remove_entity_effect_packet(&mut self, entity_id: i32, effect: Effect) {
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The entity does not exist (from remove effect packet)");
                return;
            }
        };
        entity.remove_effect(effect);
    }

    pub fn handle_entity_metadata_packet(&mut self, entity_id: i32, new_metadata: EntityMetadata) {
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The entity does not exist (from entity metadata packet)");
                return;
            }
        };
        match entity {
            Entity::ExperienceOrb { metadata, .. }
            | Entity::LivingEntity { metadata, .. }
            | Entity::OtherEntity { metadata, .. }
            | Entity::Painting { metadata, .. }
            | Entity::Player { metadata, .. } => {
                for (index, value) in new_metadata.items.into_iter() {
                    metadata.insert(index, value);
                }
            }
        }
    }

    pub fn handle_entity_velocity_packet(&mut self, entity_id: i32, velocity_x: i16, velocity_y: i16, velocity_z: i16) {
        let new_velocity_x = velocity_x as f64 / 8000.0;
        let new_velocity_y = velocity_y as f64 / 8000.0;
        let new_velocity_z = velocity_z as f64 / 8000.0;
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The entity does not exist (from entity velocity packet)");
                return;
            }
        };
        entity.set_velocity(new_velocity_x, new_velocity_y, new_velocity_z);
    }

    pub fn handle_entity_equipement_packet(&mut self, entity_id: i32, equipment: minecraft_format::slots::EquipmentSlotArray) {
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The entity does not exist (from entity equipment packet)");
                return;
            }
        };
        let entity_equipment = match entity.get_mut_equipment() {
            Some(entity_equipment) => entity_equipment,
            None => return,
        };
        for (place, slot) in equipment.slots.into_iter() {
            match place {
                minecraft_format::slots::EquipmentSlot::MainHand => entity_equipment.main_hand = slot,
                minecraft_format::slots::EquipmentSlot::OffHand => entity_equipment.off_hand = slot,
                minecraft_format::slots::EquipmentSlot::Boots => entity_equipment.boots = slot,
                minecraft_format::slots::EquipmentSlot::Leggings => entity_equipment.leggings = slot,
                minecraft_format::slots::EquipmentSlot::Chestplate => entity_equipment.chestplate = slot,
                minecraft_format::slots::EquipmentSlot::Helmet => entity_equipment.helmet = slot,
            }
        }
    }

    pub fn handle_teleport_entity_packet(&mut self, entity_id: i32, x: f64, y: f64, z: f64, yaw: u8, pitch: u8, _on_ground: bool) {
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The entity does not exist (from teleport entity packet)");
                return;
            }
        };
        entity.set_position(x, y, z);
        entity.set_rotation(yaw, pitch);
    }

    pub fn handle_entity_attributes_packet<'a>(
        &mut self,
        entity_id: i32,
        attributes: minecraft_format::packets::Map<'a, minecraft_format::packets::Identifier<'a>, minecraft_format::entity::EntityAttribute<'a>, i32>,
    ) {
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The entity does not exist (from entity attributes packet)({})", entity_id);
                return;
            }
        };
        let entity_attributes = match entity.get_mut_attributes() {
            Some(entity_attributes) => entity_attributes,
            None => return,
        };
        for (key, value) in attributes.items {
            entity_attributes.insert(key.to_string(), (value.value, value.modifiers.items));
        }
    }

    pub fn handle_entity_effect_packet(&mut self, entity_id: i32, effect: Effect, amplifier: u8, duration: i32) {
        let entity = match self.entities.get_mut(&entity_id) {
            Some(entity) => entity,
            None => {
                warn!("The entity does not exist (from entity attributes packet)");
                return;
            }
        };
        entity.add_effect(effect, amplifier + 1);
    }
}
