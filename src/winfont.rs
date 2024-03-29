use com_wrapper::ComWrapper;
use dcommon::Error;
use winapi::shared::winerror::SUCCEEDED;
use winapi::um::dwrite::{DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED};
use winapi::um::unknwnbase::IUnknown;
use winapi::Interface;
use wio::com::ComPtr;

#[repr(transparent)]
#[derive(Clone, ComWrapper)]
#[com(send, sync, debug)]
pub struct Factory {
    ptr: ComPtr<IDWriteFactory>,
}

// stolen from: https://github.com/Connicpu/directwrite-rs/tree/master

impl Factory {
    pub fn new() -> Result<Factory, Error> {
        unsafe {
            let mut ptr: *mut IDWriteFactory = std::ptr::null_mut();
            let hr = DWriteCreateFactory(
                DWRITE_FACTORY_TYPE_SHARED,
                &IDWriteFactory::uuidof(),
                &mut ptr as *mut _ as *mut *mut IUnknown,
            );

            if SUCCEEDED(hr) {
                Ok(Factory::from_raw(ptr))
            } else {
                Err(hr.into())
            }
        }
    }
}

// not sure what i need these for but:
pub unsafe trait IFactory {
    unsafe fn raw_f(&self) -> &IDWriteFactory;
}

unsafe impl IFactory for Factory {
    unsafe fn raw_f(&self) -> &IDWriteFactory {
        &self.ptr
    }
}
