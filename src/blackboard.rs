use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect)]
pub struct BlackboardKeyId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum BlackboardValueType {
    F32,
    I32,
    Bool,
    Entity,
    Vec2,
    Vec3,
    String,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum BlackboardValue {
    F32(f32),
    I32(i32),
    Bool(bool),
    Entity(Entity),
    Vec2(Vec2),
    Vec3(Vec3),
    String(String),
}

impl BlackboardValue {
    pub fn value_type(&self) -> BlackboardValueType {
        match self {
            Self::F32(_) => BlackboardValueType::F32,
            Self::I32(_) => BlackboardValueType::I32,
            Self::Bool(_) => BlackboardValueType::Bool,
            Self::Entity(_) => BlackboardValueType::Entity,
            Self::Vec2(_) => BlackboardValueType::Vec2,
            Self::Vec3(_) => BlackboardValueType::Vec3,
            Self::String(_) => BlackboardValueType::String,
        }
    }
}

impl From<f32> for BlackboardValue {
    fn from(value: f32) -> Self {
        Self::F32(value)
    }
}

impl From<i32> for BlackboardValue {
    fn from(value: i32) -> Self {
        Self::I32(value)
    }
}

impl From<bool> for BlackboardValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Entity> for BlackboardValue {
    fn from(value: Entity) -> Self {
        Self::Entity(value)
    }
}

impl From<Vec2> for BlackboardValue {
    fn from(value: Vec2) -> Self {
        Self::Vec2(value)
    }
}

impl From<Vec3> for BlackboardValue {
    fn from(value: Vec3) -> Self {
        Self::Vec3(value)
    }
}

impl From<String> for BlackboardValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for BlackboardValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct BlackboardKeyDefinition {
    pub id: BlackboardKeyId,
    pub name: String,
    pub value_type: BlackboardValueType,
    pub required: bool,
    pub default_value: Option<BlackboardValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlackboardError {
    UnknownKey(BlackboardKeyId),
    TypeMismatch {
        expected: BlackboardValueType,
        actual: BlackboardValueType,
    },
}

#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Blackboard {
    pub values: Vec<Option<BlackboardValue>>,
    pub declared_types: Vec<Option<BlackboardValueType>>,
    pub revision: u64,
    pub dirty_keys: Vec<BlackboardKeyId>,
}

impl Blackboard {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: vec![None; capacity],
            declared_types: vec![None; capacity],
            revision: 0,
            dirty_keys: Vec::new(),
        }
    }

    pub fn from_schema(schema: &[BlackboardKeyDefinition]) -> Self {
        let mut blackboard = Self::with_capacity(schema.len());
        blackboard.ensure_schema(schema);
        blackboard
    }

    pub fn ensure_schema(&mut self, schema: &[BlackboardKeyDefinition]) {
        if self.values.len() < schema.len() {
            self.values.resize(schema.len(), None);
            self.declared_types.resize(schema.len(), None);
        }

        for definition in schema {
            let index = definition.id.0 as usize;
            self.declared_types[index] = Some(definition.value_type);
            if self.values[index].is_none() {
                self.values[index] = definition.default_value.clone();
            }
        }
    }

    pub fn declared_type(
        &self,
        key: BlackboardKeyId,
    ) -> Result<Option<BlackboardValueType>, BlackboardError> {
        self.declared_types
            .get(key.0 as usize)
            .copied()
            .ok_or(BlackboardError::UnknownKey(key))
    }

    pub fn contains(&self, key: BlackboardKeyId) -> bool {
        self.values.get(key.0 as usize).is_some_and(Option::is_some)
    }

    pub fn get(&self, key: BlackboardKeyId) -> Result<Option<&BlackboardValue>, BlackboardError> {
        self.values
            .get(key.0 as usize)
            .map(Option::as_ref)
            .ok_or(BlackboardError::UnknownKey(key))
    }

    pub fn remove(
        &mut self,
        key: BlackboardKeyId,
    ) -> Result<Option<BlackboardValue>, BlackboardError> {
        let value = self
            .values
            .get_mut(key.0 as usize)
            .ok_or(BlackboardError::UnknownKey(key))?
            .take();
        if value.is_some() {
            self.touch(key);
        }
        Ok(value)
    }

    pub fn set(
        &mut self,
        key: BlackboardKeyId,
        value: impl Into<BlackboardValue>,
    ) -> Result<(), BlackboardError> {
        let value = value.into();
        if let Some(expected) = self.declared_type(key)?
            && expected != value.value_type()
        {
            return Err(BlackboardError::TypeMismatch {
                expected,
                actual: value.value_type(),
            });
        }
        let slot = self
            .values
            .get_mut(key.0 as usize)
            .ok_or(BlackboardError::UnknownKey(key))?;
        if slot.as_ref() != Some(&value) {
            *slot = Some(value);
            self.touch(key);
        }
        Ok(())
    }

    pub fn changed_since(&self, revision: u64) -> bool {
        self.revision > revision
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_keys.clear();
    }

    pub fn get_f32(&self, key: BlackboardKeyId) -> Result<Option<f32>, BlackboardError> {
        self.extract(key, BlackboardValueType::F32, |value| match value {
            BlackboardValue::F32(value) => *value,
            _ => unreachable!(),
        })
    }

    pub fn get_i32(&self, key: BlackboardKeyId) -> Result<Option<i32>, BlackboardError> {
        self.extract(key, BlackboardValueType::I32, |value| match value {
            BlackboardValue::I32(value) => *value,
            _ => unreachable!(),
        })
    }

    pub fn get_bool(&self, key: BlackboardKeyId) -> Result<Option<bool>, BlackboardError> {
        self.extract(key, BlackboardValueType::Bool, |value| match value {
            BlackboardValue::Bool(value) => *value,
            _ => unreachable!(),
        })
    }

    pub fn get_entity(&self, key: BlackboardKeyId) -> Result<Option<Entity>, BlackboardError> {
        self.extract(key, BlackboardValueType::Entity, |value| match value {
            BlackboardValue::Entity(value) => *value,
            _ => unreachable!(),
        })
    }

    pub fn get_vec2(&self, key: BlackboardKeyId) -> Result<Option<Vec2>, BlackboardError> {
        self.extract(key, BlackboardValueType::Vec2, |value| match value {
            BlackboardValue::Vec2(value) => *value,
            _ => unreachable!(),
        })
    }

    pub fn get_vec3(&self, key: BlackboardKeyId) -> Result<Option<Vec3>, BlackboardError> {
        self.extract(key, BlackboardValueType::Vec3, |value| match value {
            BlackboardValue::Vec3(value) => *value,
            _ => unreachable!(),
        })
    }

    pub fn get_string(&self, key: BlackboardKeyId) -> Result<Option<&str>, BlackboardError> {
        match self.get(key)? {
            Some(BlackboardValue::String(value)) => Ok(Some(value.as_str())),
            Some(other) => Err(BlackboardError::TypeMismatch {
                expected: BlackboardValueType::String,
                actual: other.value_type(),
            }),
            None => Ok(None),
        }
    }

    fn extract<T: Copy>(
        &self,
        key: BlackboardKeyId,
        expected: BlackboardValueType,
        mapper: impl FnOnce(&BlackboardValue) -> T,
    ) -> Result<Option<T>, BlackboardError> {
        match self.get(key)? {
            Some(value) if value.value_type() == expected => Ok(Some(mapper(value))),
            Some(other) => Err(BlackboardError::TypeMismatch {
                expected,
                actual: other.value_type(),
            }),
            None => Ok(None),
        }
    }

    fn touch(&mut self, key: BlackboardKeyId) {
        self.revision = self.revision.saturating_add(1);
        if !self.dirty_keys.contains(&key) {
            self.dirty_keys.push(key);
        }
    }
}

#[cfg(test)]
#[path = "blackboard_tests.rs"]
mod tests;
