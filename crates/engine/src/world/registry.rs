use bevy::prelude::*;

pub trait RegistryGenerato<Args>: FnMut<Args> {}
pub trait RegistryName: Default + Clone + Debug + PartialEq + Eq + Hash + Component + Reflect  {}
pub trait RegistryId: Into<RegistryItemId> {}

#[derive(Default, Component, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegistryItemId {
    Empty,
    Basic(u32),
    Dynamic(u32),
}

#[derive(Default)]
pub struct Registry<Generator: RegistryGenerator, Name: RegistryName, Id: RegistryId> {
    pub basic_entities: Vec<Entity>,
    pub dynamic_generators: Vec<Box<dyn Generator>>,
    //block ids may not be stable across program runs
    pub id_map: HashMap<Name, Id>
}

impl<Generator: RegistryGenerator, Name: RegistryName, Id: RegistryId> Registry<Generator, Name, Id> {
    //inserts the corresponding Id component on the block
    pub fn add_basic(&mut self, name: Name, entity: Entity, commands: &mut Commands) {
        info!("added block {:?}", name);
        let id = Id::Basic(self.basic_entities.len() as u32);
        commands.entity(entity).insert(id);
        self.basic_entities.push(entity);
        self.id_map.insert(name, id);
    }
    pub fn add_dynamic(&mut self, name: Name, generator: Box<dyn BlockGenerator>) {
        let id = Id::Dynamic(self.dynamic_generators.len() as u32);
        self.dynamic_generators.push(generator);
        self.id_map.insert(name, id);
    }
    // pub fn create_basic(&mut self, name: Name, mesh: BlockMesh, physics: BlockPhysics, commands: &mut Commands) -> Entity{
    //     let entity = commands.spawn((name.clone(), mesh, physics)).id();
    //     self.add_basic(name, entity, commands);
    //     entity
    // }
    pub fn get_basic(&self, name: &Name) -> Option<Entity> {
        let id = self.id_map.get(&name)?;
        match id {
            Id::Basic(id) => self.basic_entities.get(*id as usize).copied(),
            _ => None
        }
    }
    pub fn get_id(&self, name: &Name) -> Id {
        match self.id_map.get(name) {
            Some(id) => *id,
            None => {
                error!("Couldn't find block id for name {:?}", name);
                Id::Empty
            },
        }
    }
    pub fn get_entity(&self, block_id: Id, position: BlockCoord, commands: &mut Commands) -> Option<Entity> {
        match block_id {
            Id::Empty => None,
            Id::Basic(id) => self.basic_entities.get(id as usize).copied(),
            Id::Dynamic(id) => self.dynamic_generators.get(id as usize).and_then(|gen| {
                let id = Self::setup_block(block_id, commands);
                gen.generate(id, position, commands);
                Some(id)
            }),
        }
    }
    fn setup_block(id: Id, commands: &mut Commands) -> Entity {
        commands.spawn(id).id()
    }
    pub fn get_block_type(&self, id: Id, position: BlockCoord, commands: &mut Commands) -> BlockType {
        match self.get_entity(id, position, commands) {
            Some(id) => BlockType::Filled(id),
            None => BlockType::Empty,
        }
    }
    pub fn remove_entity(id_query: &Query<&Id>, b: BlockType, commands: &mut Commands) {
        match b {
            BlockType::Filled(entity) => match id_query.get(entity) {
                Ok(Id::Empty) | Ok(Id::Basic(_)) | Err(_) => {},
                Ok(Id::Dynamic(_)) => commands.entity(entity).despawn_recursive(),
            },
            BlockType::Empty => {}
        }
    }
}