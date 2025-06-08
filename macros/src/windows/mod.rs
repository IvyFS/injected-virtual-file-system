#[crabtime::function]
#[macro_export]
pub fn generate_patch(
  pattern!(
    $name:literal,
    ($($arg:ty),+)
    $(-> $ret:ty)?
    $(, $detour:path)?
  ): _,
) {
  const DASH: char = '\u{002D}';
  const RETURN: &str = "\u{002D}>";
  const QUOTE: char = '\u{0022}';
  const SPACE: char = '\u{0020}';
  // const LIFETIME_A: &str = "\u{003C}'a\u{003E}";

  let function_name: String = expand!($name).to_owned();

  let snake_name: String = function_name
    .chars()
    .enumerate()
    .flat_map(|(idx, c)| {
      match c.is_uppercase() {
        true if idx > 0 => [Some('_'), Some(c.to_ascii_lowercase())],
        true => [Some(c.to_ascii_lowercase()), None],
        false => [Some(c), None],
      }
      .into_iter()
      .flatten()
    })
    .collect();
  let scream_snake_name = snake_name.to_ascii_uppercase();

  let args_raw = stringify!($(BINDING: $arg),+).replace('\n', " ");
  let (args, bindings, types) = args_raw
    .split(',')
    .enumerate()
    .map(|(idx, raw)| {
      let binding = format!("_{}", idx);
      (
        raw.replace("BINDING", &binding),
        binding,
        raw.split_once("BINDING :").unwrap().1.to_owned(),
      )
    })
    .reduce(|(args, bindings, types), (arg, binding, typ)| {
      (
        format!("{args}, {arg}"),
        format!("{bindings}, {binding}"),
        format!("{types}, {typ}"),
      )
    })
    .unwrap();

  let target_func_signature = crabtime::quote! {
    type {{function_name}}Func = unsafe extern "system" fn(
      {{types}}
    )$({{RETURN}} $ret)?
  };

  let original_lock = crabtime::quote!(ORIGINAL_{{scream_snake_name}});

  let detour_param = crabtime::quote! {$($detour)?};
  let (detour, detour_fn_name) = if detour_param == "default " || detour_param.trim().is_empty() {
    let debug = crabtime::quote! { {{QUOTE}}{{function_name}}{{QUOTE}} };
    let output = crabtime::quote! {
      unsafe extern "system" fn detour_{{function_name}}(
        {{args}}
      )$({{RETURN}} $ret)? {
        log(Message::DebugDefaultIntercept({{debug}}.to_owned()));

        let original = {{original_lock}}
          .get()
          .expect({{QUOTE}}Get original{{SPACE}}{{function_name}}{{SPACE}}function ptr{{QUOTE}})
          .lock()
          .expect({{QUOTE}}Lock mutex on{{SPACE}}{{function_name}}{{SPACE}}ptr{{QUOTE}});

        unsafe {
          original({{bindings}})
        }
      }
    };
    (output, crabtime::quote! {detour_{{function_name}}})
  } else {
    (String::new(), detour_param)
  };

  crabtime::output! {
    pub mod {{snake_name}} {
      use std::{
        os::raw::c_void,
        ptr::null_mut,
        sync::{Mutex, OnceLock},
      };

      use frida_gum::{Gum, Module, NativePointer, interceptor::Interceptor};
      use shared_types::{ErrorContext, HookError, Message, unsafe_types::UnsafeSyncCell};

      use crate::log::*;

      use super::*;

      {{detour}}

      {{target_func_signature}};

      pub static {{SPACE}}{{original_lock}}: UnsafeSyncCell<{{function_name}}Func> =
        UnsafeSyncCell::new({{detour_fn_name}});

      pub(super) unsafe fn get_original<'a>() {{RETURN}} &'a{{SPACE}}{{function_name}}Func {
        &*{{original_lock}}.get()
      }

      pub fn {{snake_name}}(gum: &Gum, module: &Module, _: &str) {{RETURN}} Result<(), HookError> {
        let mut interceptor = Interceptor::obtain(&gum);

        let export =
          module
            .find_export_by_name($name)
            .ok_or_else(|| HookError::FunctionNotFound {
              function: $name.to_owned(),
              module: module.name(),
            })?;

        if export.is_null() {
          return Err(HookError::FunctionPtrNull {
            function: $name.to_owned(),
            module: module.name(),
          });
        }

        let original = interceptor
          .replace(
            export,
            NativePointer({{detour_fn_name}} as *mut c_void),
            NativePointer(std::ptr::null_mut()),
          )
          .with_context({{QUOTE}}Failed to replace{{SPACE}}{{function_name}}{{QUOTE}})?;

        // let transmutate = || unsafe { std::mem::transmute(original) };
        // let ptr_ref = {{original_lock}}.get_or_init(|| Mutex::new(transmutate()));
        // *ptr_ref.lock()? = transmutate();
        unsafe {
          *{{original_lock}}.get() = std::mem::transmute(original);
        }

        Ok(())
      }
    }
  }
}
