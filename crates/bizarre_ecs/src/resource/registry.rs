use std::{
    any::TypeId,
    collections::{btree_map, BTreeMap, VecDeque},
    sync::{
        MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
    },
};

use super::resource::{IntoResource, Resource, ResourceData, ResourceError};

#[derive(Default)]
pub struct ResourceRegistry {
    resources: Vec<Option<Resource>>,
    type_map: BTreeMap<TypeId, usize>,
    index_dumpster: VecDeque<usize>,
}

pub type ResourceReadLock<'a, T> = MappedRwLockReadGuard<'a, T>;
pub type ResourceWriteLock<'a, T> = MappedRwLockWriteGuard<'a, T>;
pub type ResourceResult<R> = Result<R, ResourceError>;

impl ResourceRegistry {
    pub fn insert<T>(&mut self, resource: T) -> ResourceResult<()>
    where
        T: IntoResource + 'static,
    {
        let type_id = TypeId::of::<T>();
        if let btree_map::Entry::Vacant(e) = self.type_map.entry(type_id) {
            let index = if let Some(index) = self.index_dumpster.pop_front() {
                self.resources[index] = Some(resource.into_resource());
                index
            } else {
                let index = self.resources.len();
                self.resources.push(Some(resource.into_resource()));
                index
            };
            e.insert(index);
            Ok(())
        } else {
            Err(ResourceError::already_present::<T>())
        }
    }

    pub fn remove<T>(&mut self) -> ResourceResult<T>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        match self.type_map.remove(&type_id) {
            Some(index) => {
                self.index_dumpster.push_back(index);
                let res = self.resources[index].take().unwrap().into_inner();

                Ok(res)
            }
            None => Err(ResourceError::not_present::<T>()),
        }
    }

    pub fn get<T>(&self) -> ResourceResult<ResourceReadLock<T>>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();

        let index = *self
            .type_map
            .get(&type_id)
            .ok_or(ResourceError::not_present::<T>())?;

        let lock = self.resources[index]
            .as_ref()
            .ok_or(ResourceError::not_present::<T>())?;

        let guard = lock.read().unwrap();

        let guard = RwLockReadGuard::try_map(guard, |r| r.as_ref().ok());

        match guard {
            Err(_) => Err(ResourceError::cannot_convert::<T>(&lock.read().unwrap())),
            Ok(guard) => Ok(guard),
        }
    }

    pub fn get_mut<T>(&self) -> ResourceResult<ResourceWriteLock<T>>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();

        let index = *self
            .type_map
            .get(&type_id)
            .ok_or(ResourceError::not_present::<T>())?;

        let lock = self.resources[index]
            .as_ref()
            .ok_or(ResourceError::not_present::<T>())?;

        let guard = lock.write().unwrap();

        let guard = RwLockWriteGuard::try_map(guard, |r| r.as_mut().ok());

        match guard {
            Err(_) => Err(ResourceError::cannot_convert::<T>(&lock.read().unwrap())),
            Ok(guard) => Ok(guard),
        }
    }

    pub fn with_resource<T, F>(&self, func: F) -> ResourceResult<()>
    where
        T: 'static,
        F: FnOnce(&T) -> (),
    {
        let resource = self.get()?;
        func(&resource);
        Ok(())
    }

    pub fn with_resource_mut<T, F>(&self, func: F) -> ResourceResult<()>
    where
        T: 'static,
        F: FnOnce(&mut T) -> (),
    {
        let mut resource = self.get_mut::<T>()?;
        func(&mut resource);
        Ok(())
    }
}
