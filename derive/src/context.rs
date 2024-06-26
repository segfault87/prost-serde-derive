use std::cell::RefCell;
use std::fmt::Display;
use std::thread;

use quote::ToTokens;

#[derive(Default)]
pub struct Context {
    errors: RefCell<Option<Vec<syn::Error>>>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            errors: RefCell::new(Some(Vec::new())),
        }
    }

    pub fn push_error_spanned_by<A: ToTokens, T: Display>(&self, obj: A, msg: T) {
        self.errors
            .borrow_mut()
            .as_mut()
            .unwrap()
            .push(syn::Error::new_spanned(obj.into_token_stream(), msg));
    }

    pub fn push_syn_error(&self, err: syn::Error) {
        self.errors.borrow_mut().as_mut().unwrap().push(err);
    }

    pub fn check(self) -> Result<(), Vec<syn::Error>> {
        let errors = self.errors.borrow_mut().take().unwrap();
        match errors.len() {
            0 => Ok(()),
            _ => Err(errors),
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if !thread::panicking() && self.errors.borrow().as_ref().is_some_and(|e| !e.is_empty()) {
            panic!("forgot to check for errors");
        }
    }
}
