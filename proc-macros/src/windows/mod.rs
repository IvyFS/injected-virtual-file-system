use unsynn::{
  Colon, Comma, CommaDelimitedVec, Cons, Delimited, DelimitedVec, Either, Except, Gt, IParse,
  Ident, Lt, Many, Nothing, ParenthesisGroupContaining, RArrow, Span, ToTokens, TokenIter,
  TokenStream, TokenTree, unsynn,
};

use crate::windows::output::{
  DefaultDetour, FnImplArg, Mod, OriginalFnPtrCell, Output, Patcher, TargetSignature,
};

mod output;

pub(crate) type VerbatimUntil<C> = Many<Cons<Except<C>, AngleTokenTree>>;
pub(crate) type Typ = VerbatimUntil<Comma>;

unsynn! {
  #[derive(Clone)]
  pub struct AngleTokenTree(
    #[allow(clippy::type_complexity)] // look,
    pub Either<Cons<Lt, Vec<Cons<Except<Gt>, AngleTokenTree>>, Gt>, TokenTree>,
  );

  struct Input {
    name: Ident,
    _comma: Comma,
    arg_types: ParenthesisGroupContaining<CommaDelimitedVec<Typ>>,
    returns: Option<Cons<RArrow, Typ>>,
    detour: Option<Cons<Comma, Ident>>
  }
}

pub(crate) fn generate_patch(input: TokenStream) -> TokenStream {
  let input = match input.to_token_iter().parse::<Input>() {
    Ok(input) => input,
    Err(err) => panic!("Failed to parse input: {err}"),
  };

  let target_name = input.name.to_string();
  let snake_name = snake_case(&target_name);

  let args: Vec<Arg> = input
    .arg_types
    .content
    .0
    .into_iter()
    .enumerate()
    .map(|(idx, delimited)| {
      let binding = format!("_{idx}");
      Arg::new(binding, delimited.value)
    })
    .collect();

  let returns = input.returns.as_ref().map(|returns| returns.second.clone());

  let (detour_name, default_detour) = if let Some(detour) = input.detour {
    (detour.second, None)
  } else {
    let default_name = Ident::new(&format!("detour_{}", input.name), Span::call_site());
    let fn_impl_args: CommaDelimitedVec<FnImplArg> =
      map_args_to_comma_delim_vec(&args, Arg::as_fn_impl_arg);
    let bindings: CommaDelimitedVec<Ident> = map_args_to_comma_delim_vec(&args, Arg::get_ident);
    (
      default_name.clone(),
      Some(DefaultDetour::new(
        default_name,
        fn_impl_args,
        returns.clone(),
        bindings,
      )),
    )
  };

  let target_sig_name = Ident::new(&format!("{}Func", &target_name), Span::call_site());
  let target_sig_args: CommaDelimitedVec<Typ> = map_args_to_comma_delim_vec(&args, Arg::get_typ);
  let target_signature = TargetSignature::new(target_sig_name.clone(), target_sig_args, returns);

  let cell_name = Ident::new(
    &format!("ORIGINAL_{}", snake_name.to_ascii_uppercase()),
    Span::call_site(),
  );
  let original_fn_cell =
    OriginalFnPtrCell::new(cell_name.clone(), target_sig_name, detour_name.clone());

  let name = Ident::new(&snake_name, Span::call_site());
  let patcher = Patcher::new(name.clone(), target_name, detour_name, cell_name);

  let module = Mod::new(
    name,
    default_detour,
    target_signature,
    original_fn_cell,
    patcher,
  );

  Output::new(module).to_token_stream()
}

struct Arg {
  name: Ident,
  typ: Typ,
}

impl Arg {
  fn new(name: String, typ: Typ) -> Self {
    Self {
      name: Ident::new(&name, Span::call_site()),
      typ,
    }
  }

  fn get_ident(&self) -> Ident {
    self.name.clone()
  }

  fn get_typ(&self) -> Typ {
    self.typ.clone()
  }

  fn as_fn_impl_arg(&self) -> FnImplArg {
    Cons {
      first: self.name.clone(),
      second: Colon::new(),
      third: self.typ.clone(),
      fourth: Nothing,
    }
  }
}

fn map_args_to_comma_delim_vec<T>(args: &[Arg], map: impl Fn(&Arg) -> T) -> CommaDelimitedVec<T> {
  let inner = args
    .iter()
    .map(|arg| Delimited {
      value: map(arg),
      delimiter: Some(Comma::new()),
    })
    .collect();
  DelimitedVec(inner)
}

fn snake_case(input: impl AsRef<str>) -> String {
  input
    .as_ref()
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
    .collect()
}

#[cfg(test)]
mod test {
  use unsynn::ToTokens;

  use crate::windows::generate_patch;

  #[test]
  fn parse_test() {
    let input = "\
FindFirstFileExW,
(
  PCWSTR,
  FINDEX_INFO_LEVELS,
  *mut c_void,
  FINDEX_SEARCH_OPS,
  *const c_void,
  u32
) -> HANDLE";

    let output = generate_patch(input.into_token_stream());

    eprintln!("{output}")
  }
}
