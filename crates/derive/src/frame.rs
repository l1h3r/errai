use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use std::borrow::Cow;
use syn::braced;
use syn::buffer::Cursor;
use syn::meta::ParseNestedMeta;
use syn::parse::Parse;
use syn::parse::ParseBuffer;
use syn::parse::ParseStream;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token;
use syn::__private::CustomToken;
use syn::token::Gt;
use syn::token::Lt;
use syn::token::Token;
use syn::Attribute;
use syn::Error;
use syn::Expr;
use syn::GenericArgument;
use syn::Ident;
use syn::Lifetime;
use syn::LitStr;
use syn::PathArguments;
use syn::PathSegment;
use syn::Result;
use syn::Token;
use syn::Type;

type FrameFields = Punctuated<FrameField, Token![,]>;

fn meta_name(meta: &ParseNestedMeta) -> String {
  meta
    .path
    .get_ident()
    .map(ToString::to_string)
    .unwrap_or_default()
}

// =============================================================================
// Frame
// =============================================================================

pub struct Frame {
  attr: FrameAttr,
  visq: Token![pub],
  root: Token![struct],
  name: Ident,
  life: Option<FrameGenerics>,
  body: token::Brace,
  data: FrameFields,
}

impl Parse for Frame {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    let content: ParseBuffer<'_>;

    Ok(Self {
      attr: input.parse()?,
      visq: input.parse()?,
      root: input.parse()?,
      name: input.parse()?,
      life: input.parse()?,
      body: braced!(content in input),
      data: Punctuated::parse_terminated(&content)?,
    })
  }
}

impl ToTokens for Frame {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let name: &Ident = &self.name;

    let object_lifetime: Option<&FrameGenerics> = self.life.as_ref();
    let static_lifetime: Option<FrameGenerics> = object_lifetime.map(FrameGenerics::to_static);
    let decode_lifetime: FrameGenerics = match object_lifetime {
      Some(ref generics) => generics.duplicate(None),
      None => FrameGenerics::new("'_"),
    };

    let accessor = self.data.iter().map(FrameAccessor);

    let owned_name = self.data.iter().map(FrameField::name);
    let owned_expr = self.data.iter().map(FrameIntoOwned);

    let decode_name = owned_name.clone();
    let decode_expr = self.data.iter().map(FrameDecoder);

    if !self.attr.skip_decoding {
      tokens.extend(quote! {
        impl #object_lifetime crate::decode::Decode #decode_lifetime for #name #object_lifetime {
          fn decode(decoder: &mut crate::decode::Decoder #decode_lifetime) -> crate::error::Result<Self> {
            Ok(Self {
              #(#decode_name: #decode_expr),*
            })
          }
        }
      });
    }

    tokens.extend(quote! {
      impl #object_lifetime crate::traits::IntoOwned for #name #object_lifetime {
        type Owned = #name #static_lifetime;

        #[inline]
        fn into_owned(self) -> Self::Owned {
          #name {
            #(#owned_name: #owned_expr),*
          }
        }
      }
    });

    tokens.extend(quote! {
      impl #object_lifetime #name #object_lifetime {
        #(#accessor)*
      }
    })
  }
}

// =============================================================================
// Frame Attributes
// =============================================================================

struct FrameAttr {
  skip_decoding: bool,
}

impl Parse for FrameAttr {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    let list: Vec<Attribute> = input.call(Attribute::parse_outer)?;

    let mut skip_decoding: bool = false;

    for attr in list {
      if !attr.path().is_ident("frame") {
        continue;
      }

      attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("skip_decoding") {
          if skip_decoding {
            return Err(Error::new(
              meta.input.span(),
              "Duplicate `skip_decoding` Attribute.",
            ));
          }

          skip_decoding = true;
        } else {
          return Err(Error::new(
            meta.path.span(),
            format!("Unknown Attribute - `{}`.", meta_name(&meta)),
          ));
        }

        Ok(())
      })?;
    }

    Ok(Self { skip_decoding })
  }
}

// =============================================================================
// Frame Generics
// =============================================================================

struct FrameGenerics {
  token_lt: Token![<],
  lifetime: Lifetime,
  token_gt: Token![>],
}

impl FrameGenerics {
  fn new(symbol: &str) -> Self {
    Self {
      token_lt: Lt {
        spans: [Span::call_site()],
      },
      lifetime: Lifetime::new(symbol, Span::call_site()),
      token_gt: Gt {
        spans: [Span::call_site()],
      },
    }
  }

  fn duplicate(&self, symbol: Option<&str>) -> Self {
    Self {
      token_lt: Lt {
        spans: self.token_lt.spans,
      },
      lifetime: Lifetime {
        apostrophe: self.lifetime.apostrophe,
        ident: if let Some(symbol) = symbol {
          Ident::new(symbol, self.lifetime.ident.span())
        } else {
          self.lifetime.ident.clone()
        },
      },
      token_gt: Gt {
        spans: self.token_gt.spans,
      },
    }
  }

  fn to_static(&self) -> Self {
    self.duplicate(Some("static"))
  }
}

impl Parse for FrameGenerics {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    Ok(Self {
      token_lt: input.parse()?,
      lifetime: input.parse()?,
      token_gt: input.parse()?,
    })
  }
}

impl ToTokens for FrameGenerics {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.token_lt.to_tokens(tokens);
    self.lifetime.to_tokens(tokens);
    self.token_gt.to_tokens(tokens);
  }
}

impl CustomToken for FrameGenerics {
  fn peek(cursor: Cursor<'_>) -> bool {
    <Token![<]>::peek(cursor)
  }

  fn display() -> &'static str {
    <Token![<]>::display()
  }
}

// =============================================================================
// Frame Field
// =============================================================================

struct FrameField {
  attr: FrameFieldAttr,
  name: Ident,
  body: Token![:],
  kind: Type,
}

impl FrameField {
  const fn name(&self) -> &Ident {
    &self.name
  }

  fn kind(&self) -> FrameType<'_> {
    FrameType::new(&self.kind)
  }

  fn info(&self) -> Cow<'_, str> {
    if let Some(info) = self.attr.info.as_deref() {
      Cow::Borrowed(info)
    } else {
      let name: String = self.name.to_string();
      let name: String = name.replace("_", " ");

      Cow::Owned(name)
    }
  }
}

impl Parse for FrameField {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    Ok(Self {
      attr: input.parse()?,
      name: input.parse()?,
      body: input.parse()?,
      kind: input.parse()?,
    })
  }
}

// =============================================================================
// Frame Decoder
// =============================================================================

struct FrameDecoder<'a>(&'a FrameField);

impl ToTokens for FrameDecoder<'_> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self.0.attr.read.as_deref() {
      Some("@latin1") => {
        tokens.extend(quote! {
          decoder.decode_latin1()?
        });
      }
      Some("@u64") => {
        tokens.extend(quote! {
          crate::utils::decode_u64_relaxed(decoder.step(0, |slice| slice.take(8)))
        });
      }
      Some(expr) => {
        panic!("Unknown Expr: {:?}", expr);
      }
      None => {
        tokens.extend(quote! {
          decoder.decode()?
        });
      }
    }
  }
}

// =============================================================================
// Frame IntoOwned
// =============================================================================

struct FrameIntoOwned<'a>(&'a FrameField);

impl ToTokens for FrameIntoOwned<'_> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let copy: bool = self.0.attr.copy;
    let name: &Ident = self.0.name();

    if copy {
      tokens.extend(quote!(self.#name));
    } else {
      tokens.extend(quote!(crate::traits::IntoOwned::into_owned(self.#name)));
    }
  }
}

// =============================================================================
// Frame Field Accessor
// =============================================================================

struct FrameAccessor<'a>(&'a FrameField);

impl ToTokens for FrameAccessor<'_> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let name: &Ident = self.0.name();
    let kind: FrameType<'_> = self.0.kind();
    let info: Cow<'_, str> = self.0.info();
    let docs: String = format!("Get the {info} of the frame.");

    let accessor: Expr = kind.accessor(name);
    let constant: Option<Token![const]> = kind.constant();

    tokens.extend(quote! {
      #[doc = #docs]
      #[inline]
      pub #constant fn #name(&self) -> #kind {
        #accessor
      }
    });
  }
}

// =============================================================================
// Frame Field Attributes
// =============================================================================

struct FrameFieldAttr {
  copy: bool,
  info: Option<String>,
  read: Option<String>,
}

impl Parse for FrameFieldAttr {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    let list: Vec<Attribute> = input.call(Attribute::parse_outer)?;

    let mut copy: bool = false;
    let mut info: Option<String> = None;
    let mut read: Option<String> = None;

    for attr in list {
      if !attr.path().is_ident("frame") {
        continue;
      }

      attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("info") {
          if info.is_some() {
            return Err(Error::new(meta.input.span(), "Duplicate `info` Attribute."));
          }

          let _ignore: Token![=] = meta.input.parse()?;
          let content: LitStr = meta.input.parse()?;

          info = Some(content.value());
        } else if meta.path.is_ident("copy") {
          if copy {
            return Err(Error::new(meta.input.span(), "Duplicate `copy` Attribute."));
          }

          copy = true;
        } else if meta.path.is_ident("read") {
          if read.is_some() {
            return Err(Error::new(meta.input.span(), "Duplicate `read` Attribute."));
          }

          let _ignore: Token![=] = meta.input.parse()?;
          let content: LitStr = meta.input.parse()?;

          read = Some(content.value());
        } else {
          return Err(Error::new(
            meta.path.span(),
            format!("Unknown Attribute - `{}`.", meta_name(&meta)),
          ));
        }

        Ok(())
      })?;
    }

    Ok(Self { copy, info, read })
  }
}

// =============================================================================
// Frame Field Type
// =============================================================================

enum FrameType<'a> {
  Cow(&'a Type),
  Ref(&'a Type),
}

impl<'a> FrameType<'a> {
  fn new(kind: &'a Type) -> Self {
    if let Some(kind) = Self::unwrap_cow(kind) {
      Self::Cow(kind)
    } else {
      Self::Ref(kind)
    }
  }

  fn accessor(&self, name: &Ident) -> Expr {
    match self {
      Self::Cow(_) => parse_quote!(::alloc::borrow::Borrow::borrow(&self.#name)),
      Self::Ref(_) => parse_quote!(self.#name),
    }
  }

  fn constant(&self) -> Option<Token![const]> {
    match self {
      Self::Cow(_) => None,
      Self::Ref(_) => Some(parse_quote!(const)),
    }
  }

  fn unwrap_cow(kind: &Type) -> Option<&Type> {
    let Type::Path(ref path) = kind else {
      return None;
    };

    if path.path.segments.len() != 1 {
      return None;
    }

    let segment: &PathSegment = &path.path.segments[0];

    if segment.ident != "Cow" {
      return None;
    }

    let PathArguments::AngleBracketed(ref arguments) = segment.arguments else {
      return None;
    };

    if arguments.args.len() != 2 {
      return None;
    }

    let GenericArgument::Type(ref inner) = arguments.args[1] else {
      return None;
    };

    Some(inner)
  }
}

impl ToTokens for FrameType<'_> {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      Self::Cow(inner) => {
        tokens.extend(quote!(&#inner));
      }
      Self::Ref(inner) => {
        inner.to_tokens(tokens);
      }
    }
  }
}
