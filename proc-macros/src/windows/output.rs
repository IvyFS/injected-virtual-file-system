use quote::{ToTokens, quote};
use unsynn::{Colon, CommaDelimitedVec, Cons, Ident, ToTokens as _, TokenStream};

use crate::windows::Typ;

pub(crate) struct Output {
  module: Mod,
}

impl Output {
  pub(crate) fn new(module: Mod) -> Self {
    Self { module }
  }

  pub(crate) fn to_token_stream(&self) -> TokenStream {
    self.into_token_stream()
  }
}

impl ToTokens for Output {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let module = &self.module;
    let name = &module.name;

    let output = quote! {
      pub use #name::*;

      #module
    };

    quote::ToTokens::to_tokens(&output, tokens);
  }
}

pub(crate) struct Mod {
  name: Ident,
  // _: common imports,
  default_detour: Option<DefaultDetour>,
  target_signature: TargetSignature,
  original_fn_ptr_cell: OriginalFnPtrCell,
  patcher: Patcher,
}

impl Mod {
  pub(crate) fn new(
    name: Ident,
    default_detour: Option<DefaultDetour>,
    target_signature: TargetSignature,
    original_fn_ptr_cell: OriginalFnPtrCell,
    patcher: Patcher,
  ) -> Self {
    Self {
      name,
      default_detour,
      target_signature,
      original_fn_ptr_cell,
      patcher,
    }
  }
}

impl ToTokens for Mod {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self {
      name,
      default_detour,
      target_signature,
      original_fn_ptr_cell,
      patcher,
    } = self;

    let output = quote! {
      pub mod #name {
        use frida_gum::{Gum, Module, NativePointer, interceptor::Interceptor};
        use shared_types::{ErrorContext, HookError};

        use super::*;

        #default_detour

        #target_signature

        #original_fn_ptr_cell

        #patcher
      }
    };

    quote::ToTokens::to_tokens(&output, tokens);
  }
}

pub(crate) type FnImplArg = Cons<Ident, Colon, Typ>;

pub(crate) struct DefaultDetour {
  fn_name: Ident,
  args: TokenStream,
  returns: Option<TokenStream>,
  bindings: TokenStream,
}

impl DefaultDetour {
  pub(crate) fn new(
    fn_name: Ident,
    args: CommaDelimitedVec<FnImplArg>,
    returns: Option<Typ>,
    bindings: CommaDelimitedVec<Ident>,
  ) -> Self {
    Self {
      fn_name,
      args: args.to_token_stream(),
      returns: map_return_fragment(returns),
      bindings: bindings.to_token_stream(),
    }
  }
}

fn map_return_fragment(returns: Option<Typ>) -> Option<TokenStream> {
  returns.map(|returns| {
    let returns = returns.to_token_stream();
    quote! {-> #returns}
  })
}

impl ToTokens for DefaultDetour {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self {
      fn_name,
      args,
      returns,
      bindings,
    } = self;
    let debug_message = fn_name.to_string();

    let output = quote! {
      unsafe extern "system" fn #fn_name(#args) #returns {
        crate::log::log(shared_types::Message::DebugDefaultIntercept(#debug_message.to_owned()));

        let original = original();

        unsafe {
          original(#bindings)
        }
      }
    };

    quote::ToTokens::to_tokens(&output, tokens);
  }
}

pub(crate) struct TargetSignature {
  type_name: Ident,
  args: TokenStream,
  returns: Option<TokenStream>,
}

impl TargetSignature {
  pub(crate) fn new(type_name: Ident, args: CommaDelimitedVec<Typ>, returns: Option<Typ>) -> Self {
    Self {
      type_name,
      args: args.to_token_stream(),
      returns: map_return_fragment(returns),
    }
  }
}

impl ToTokens for TargetSignature {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self {
      type_name,
      args,
      returns,
    } = self;

    let output = quote! {
      type #type_name = unsafe extern "system" fn(#args) #returns;
    };

    quote::ToTokens::to_tokens(&output, tokens);
  }
}

pub(crate) struct OriginalFnPtrCell {
  name: Ident,
  target_sig_name: Ident,
  detour_name: Ident,
}

impl OriginalFnPtrCell {
  pub(crate) fn new(name: Ident, target_sig_name: Ident, detour_name: Ident) -> Self {
    Self {
      name,
      target_sig_name,
      detour_name,
    }
  }
}

impl ToTokens for OriginalFnPtrCell {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self {
      name,
      target_sig_name,
      detour_name,
    } = self;

    let output = quote! {
      pub static #name: shared_types::unsafe_types::UnsafeSyncCell<#target_sig_name> =
        shared_types::unsafe_types::UnsafeSyncCell::new(#detour_name);

      pub unsafe fn original<'a>() -> &'a #target_sig_name {
        &*#name.get()
      }
    };

    quote::ToTokens::to_tokens(&output, tokens);
  }
}

pub(crate) struct Patcher {
  name: Ident,
  target_name: String,
  detour_name: Ident,
  original_fn_ptr_cell: Ident,
}

impl Patcher {
  pub(crate) fn new(
    name: Ident,
    target_name: String,
    detour_name: Ident,
    original_fn_ptr_cell: Ident,
  ) -> Self {
    Self {
      name,
      target_name,
      detour_name,
      original_fn_ptr_cell,
    }
  }
}

impl ToTokens for Patcher {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self {
      name,
      target_name,
      detour_name,
      original_fn_ptr_cell,
    } = self;
    let target_name_str = target_name.to_string();
    let interceptor_err_msg = format!("Failed to replace {target_name}");

    let output = quote! {
      pub fn #name(gum: &Gum, module: &Module, _: &str) -> Result<(), HookError> {
        let mut interceptor = Interceptor::obtain(&gum);

        let export = module
          .find_export_by_name(#target_name)
          .ok_or_else(|| HookError::FunctionNotFound {
            function: #target_name_str.to_owned(),
            module: module.name(),
          })?;

        if export.is_null() {
          return Err(HookError::FunctionPtrNull {
            function: #target_name_str.to_owned(),
            module: module.name(),
          });
        }

        let original = interceptor
          .replace(
            export,
            NativePointer(#detour_name as *mut std::ffi::c_void),
            NativePointer(std::ptr::null_mut()),
          )
          .with_context(|| #interceptor_err_msg)?;

        unsafe {
          *#original_fn_ptr_cell.get() = std::mem::transmute(original);
        }

        Ok(())
      }
    };

    quote::ToTokens::to_tokens(&output, tokens);
  }
}
