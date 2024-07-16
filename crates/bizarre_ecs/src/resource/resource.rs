use std::{
    alloc::Layout,
    any::{type_name, TypeId},
};

use thiserror::Error;

#[derive(Debug)]
pub struct RegisteredResource {
    type_id: TypeId,
    data: *mut (),
    size: usize,
    type_name: &'static str,
}

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error(r#"Cannot convert RegisteredResource of type "{expected}" to "{found}""#)]
    CannotConvert {
        expected: &'static str,
        found: &'static str,
    },

    #[error(r#"Resource "{type_name}" is already present in this registry"#)]
    AlreadyPresent { type_name: &'static str },

    #[error(r#"Resource "{type_name}" is not present in this registry"#)]
    NotPresent { type_name: &'static str },
}

impl ResourceError {
    pub fn cannot_convert<T>(resource: &RegisteredResource) -> ResourceError {
        Self::CannotConvert {
            expected: resource.type_name,
            found: type_name::<T>(),
        }
    }

    pub fn already_present<T>() -> ResourceError {
        Self::AlreadyPresent {
            type_name: type_name::<T>(),
        }
    }

    pub fn not_present<T>() -> ResourceError {
        Self::NotPresent {
            type_name: type_name::<T>(),
        }
    }
}

impl RegisteredResource {
    pub fn as_ref<T>(&self) -> Result<&T, ResourceError>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();

        if type_id != self.type_id {
            Err(ResourceError::cannot_convert::<T>(&self))
        } else {
            let obj = unsafe { std::mem::transmute(self.data) };
            Ok(obj)
        }
    }

    pub fn as_mut<T>(&mut self) -> Result<&mut T, ResourceError>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();

        if type_id != self.type_id {
            Err(ResourceError::cannot_convert::<T>(&self))
        } else {
            let obj = unsafe { std::mem::transmute(self.data) };
            Ok(obj)
        }
    }

    pub fn into_inner<T>(self) -> T
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();

        if type_id != self.type_id {
            panic!("Trying to consume RegisteredResource and convert it into inner, but the type is wrong");
        }

        let obj: T = unsafe { *Box::from_raw(self.data as *mut T) };
        obj
    }
}

pub trait IntoResource {
    fn into_resource(self) -> RegisteredResource;
}

impl<T> IntoResource for T
where
    T: 'static + Send + Sync + Sized,
{
    fn into_resource(self) -> RegisteredResource {
        let size = size_of::<Self>();
        let data = {
            let boxed = Box::new(self);
            Box::into_raw(boxed) as *mut ()
        };
        let type_id = TypeId::of::<Self>();
        let type_name = type_name::<Self>();

        let res = RegisteredResource {
            data,
            size,
            type_id,
            type_name,
        };
        res
    }
}