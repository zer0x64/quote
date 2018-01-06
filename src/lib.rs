//! Quasi-quoting without a Syntex dependency, intended for use with [Macros
//! 1.1](https://github.com/rust-lang/rfcs/blob/master/text/1681-macros-1.1.md).
//!
//! ```toml
//! [dependencies]
//! quote = "0.3"
//! ```
//!
//! ```
//! #[macro_use]
//! extern crate quote;
//! #
//! # fn main() {}
//! ```
//!
//! Interpolation is done with `#var`:
//!
//! ```
//! # #[macro_use]
//! # extern crate quote;
//! #
//! # fn main() {
//! #     let generics = "";
//! #     let where_clause = "";
//! #     let field_ty = "";
//! #     let item_ty = "";
//! #     let path = "";
//! #     let value = "";
//! #
//! let tokens = quote! {
//!     struct SerializeWith #generics #where_clause {
//!         value: &'a #field_ty,
//!         phantom: ::std::marker::PhantomData<#item_ty>,
//!     }
//!
//!     impl #generics serde::Serialize for SerializeWith #generics #where_clause {
//!         fn serialize<S>(&self, s: &mut S) -> Result<(), S::Error>
//!             where S: serde::Serializer
//!         {
//!             #path(self.value, s)
//!         }
//!     }
//!
//!     SerializeWith {
//!         value: #value,
//!         phantom: ::std::marker::PhantomData::<#item_ty>,
//!     }
//! };
//! #
//! # }
//! ```
//!
//! Repetition is done using `#(...)*` or `#(...),*` very similar to `macro_rules!`:
//!
//! - `#(#var)*` - no separators
//! - `#(#var),*` - the character before the asterisk is used as a separator
//! - `#( struct #var; )*` - the repetition can contain other things
//! - `#( #k => println!("{}", #v), )*` - even multiple interpolations
//!
//! The return type of `quote!` is `quote::Tokens`. Tokens can be interpolated into
//! other quotes:
//!
//! ```text
//! let t = quote! { /* ... */ };
//! return quote! { /* ... */ #t /* ... */ };
//! ```
//!
//! Call `to_string()` or `as_str()` on a Tokens to get a `String` or `&str` of Rust
//! code.
//!
//! The `quote!` macro relies on deep recursion so some large invocations may fail
//! with "recursion limit reached" when you compile. If it fails, bump up the
//! recursion limit by adding `#![recursion_limit = "128"]` to your crate. An even
//! higher limit may be necessary for especially large invocations.

extern crate proc_macro2;
extern crate proc_macro;

mod tokens;
pub use tokens::Tokens;

mod to_tokens;
pub use to_tokens::{ByteStr, ToTokens};

// Not public API.
#[doc(hidden)]
pub mod __rt {
    // Not public API.
    pub use proc_macro2::*;

    // Not public API.
    pub fn parse(tokens: &mut ::Tokens, span: Span, s: &str) {
        let s: TokenStream = s.parse().expect("invalid token stream");
        tokens.append_all(s.into_iter().map(|mut t| {
            t.span = span;
            t
        }));
    }

    // Not public API.
    pub fn append_kind(tokens: &mut ::Tokens, span: Span, kind: TokenNode) {
        tokens.append(TokenTree {
            span: span,
            kind: kind,
        })
    }
}

/// The whole point.
///
/// Performs variable interpolation against the input and produces it as
/// [`Tokens`]. For returning tokens to the compiler in a procedural macro, use
/// `into()` to build a `TokenStream`.
///
/// [`Tokens`]: struct.Tokens.html
///
/// # Interpolation
///
/// Variable interpolation is done with `#var` (similar to `$var` in
/// `macro_rules!` macros). This grabs the `var` variable that is currently in
/// scope and inserts it in that location in the output tokens. The variable
/// must implement the [`ToTokens`] trait.
///
/// [`ToTokens`]: trait.ToTokens.html
///
/// Repetition is done using `#(...)*` or `#(...),*` again similar to
/// `macro_rules!`. This iterates through the elements of any variable
/// interpolated within the repetition and inserts a copy of the repetition body
/// for each one. The variables in an interpolation may be anything that
/// implements `IntoIterator`, including `Vec` or a pre-existing iterator.
///
/// - `#(#var)*` — no separators
/// - `#(#var),*` — the character before the asterisk is used as a separator
/// - `#( struct #var; )*` — the repetition can contain other tokens
/// - `#( #k => println!("{}", #v), )*` — even multiple interpolations
///
/// # Hygiene
///
/// Any interpolated tokens preserve the `Span` information provided by their
/// `ToTokens` implementation. Tokens that originate within the `quote!`
/// invocation are spanned with [`Span::def_site()`].
///
/// [`Span::def_site()`]: https://docs.rs/proc-macro2/0.1/proc_macro2/struct.Span.html#method.def_site
///
/// A different span can be provided through the [`quote_spanned!`] macro.
///
/// [`quote_spanned!`]: macro.quote_spanned.html
///
/// # Example
///
/// ```
/// extern crate proc_macro;
///
/// #[macro_use]
/// extern crate quote;
///
/// use proc_macro::TokenStream;
///
/// # const IGNORE_TOKENS: &'static str = stringify! {
/// #[proc_macro_derive(HeapSize)]
/// # };
/// pub fn derive_heap_size(input: TokenStream) -> TokenStream {
///     // Parse the input and figure out what implementation to generate...
///     # const IGNORE_TOKENS: &'static str = stringify! {
///     let name = /* ... */;
///     let expr = /* ... */;
///     # };
///     #
///     # let name = 0;
///     # let expr = 0;
///
///     let expanded = quote! {
///         // The generated impl.
///         impl ::heapsize::HeapSize for #name {
///             fn heap_size_of_children(&self) -> usize {
///                 #expr
///             }
///         }
///     };
///
///     // Hand the output tokens back to the compiler.
///     expanded.into()
/// }
/// #
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! quote {
    ($($tt:tt)*) => (quote_spanned!($crate::__rt::Span::def_site()=> $($tt)*));
}

/// Same as `quote!`, but applies a given span to all tokens originating within
/// the macro invocation.
///
/// # Syntax
///
/// A span expression of type [`Span`], followed by `=>`, followed by the tokens
/// to quote. The span expression should be brief -- use a variable for anything
/// more than a few characters. There should be no space before the `=>` token.
///
/// [`Span`]: https://docs.rs/proc-macro2/0.1/proc_macro2/struct.Span.html
///
/// ```
/// # #[macro_use]
/// # extern crate quote;
/// # extern crate proc_macro2;
/// #
/// # use proc_macro2::Span;
/// #
/// # fn main() {
/// # const IGNORE_TOKENS: &'static str = stringify! {
/// let span = /* ... */;
/// # };
/// # let span = Span::call_site();
/// # let init = 0;
///
/// // On one line, use parentheses.
/// let tokens = quote_spanned!(span=> Box::into_raw(Box::new(#init)));
///
/// // On multiple lines, place the span at the top and use braces.
/// let tokens = quote_spanned! {span=>
///     Box::into_raw(Box::new(#init))
/// };
/// # }
/// ```
///
/// # Hygiene
///
/// Any interpolated tokens preserve the `Span` information provided by their
/// `ToTokens` implementation. Tokens that originate within the `quote_spanned!`
/// invocation are spanned with the given span argument.
///
/// # Example
///
/// The following procedural macro code uses `quote_spanned!` to assert that a
/// particular Rust type implements the [`Sync`] trait so that references can be
/// safely shared between threads.
///
/// [`Sync`]: https://doc.rust-lang.org/std/marker/trait.Sync.html
///
/// ```
/// # #[macro_use]
/// # extern crate quote;
/// # extern crate proc_macro2;
/// #
/// # use quote::{Tokens, ToTokens};
/// # use proc_macro2::Span;
/// #
/// # struct Type;
/// #
/// # impl Type {
/// #     fn span(&self) -> Span {
/// #         Span::call_site()
/// #     }
/// # }
/// #
/// # impl ToTokens for Type {
/// #     fn to_tokens(&self, _tokens: &mut Tokens) {}
/// # }
/// #
/// # fn main() {
/// # let ty = Type;
/// # let def_site = Span::def_site();
/// #
/// let ty_span = ty.span().resolved_at(def_site);
/// let assert_sync = quote_spanned! {ty_span=>
///     struct _AssertSync where #ty: Sync;
/// };
/// # }
/// ```
///
/// If the assertion fails, the user will see an error like the following. The
/// input span of their type is hightlighted in the error.
///
/// ```text
/// error[E0277]: the trait bound `*const (): std::marker::Sync` is not satisfied
///   --> src/main.rs:10:21
///    |
/// 10 |     static ref PTR: *const () = &();
///    |                     ^^^^^^^^^ `*const ()` cannot be shared between threads safely
/// ```
///
/// In this example it is important for the where-clause to be spanned with the
/// line/column information of the user's input type so that error messages are
/// placed appropriately by the compiler. But it is also incredibly important
/// that `Sync` resolves at the macro definition site and not the macro call
/// site. If we resolve `Sync` at the same span that the user's type is going to
/// be resolved, then they could bypass our check by defining their own trait
/// named `Sync` that is implemented for their type.
#[macro_export]
macro_rules! quote_spanned {
    ($span:expr=> $($tt:tt)*) => {
        {
            let mut _s = $crate::Tokens::new();
            let _span = $span;
            quote_each_token!(_s _span $($tt)*);
            _s
        }
    };
}

// Extract the names of all #metavariables and pass them to the $finish macro.
//
// in:   pounded_var_names!(then () a #b c #( #d )* #e)
// out:  then!(() b d e)
#[macro_export]
#[doc(hidden)]
macro_rules! pounded_var_names {
    ($finish:ident ($($found:ident)*) # ( $($inner:tt)* ) $($rest:tt)*) => {
        pounded_var_names!($finish ($($found)*) $($inner)* $($rest)*)
    };

    ($finish:ident ($($found:ident)*) # [ $($inner:tt)* ] $($rest:tt)*) => {
        pounded_var_names!($finish ($($found)*) $($inner)* $($rest)*)
    };

    ($finish:ident ($($found:ident)*) # { $($inner:tt)* } $($rest:tt)*) => {
        pounded_var_names!($finish ($($found)*) $($inner)* $($rest)*)
    };

    ($finish:ident ($($found:ident)*) # $first:ident $($rest:tt)*) => {
        pounded_var_names!($finish ($($found)* $first) $($rest)*)
    };

    ($finish:ident ($($found:ident)*) ( $($inner:tt)* ) $($rest:tt)*) => {
        pounded_var_names!($finish ($($found)*) $($inner)* $($rest)*)
    };

    ($finish:ident ($($found:ident)*) [ $($inner:tt)* ] $($rest:tt)*) => {
        pounded_var_names!($finish ($($found)*) $($inner)* $($rest)*)
    };

    ($finish:ident ($($found:ident)*) { $($inner:tt)* } $($rest:tt)*) => {
        pounded_var_names!($finish ($($found)*) $($inner)* $($rest)*)
    };

    ($finish:ident ($($found:ident)*) $ignore:tt $($rest:tt)*) => {
        pounded_var_names!($finish ($($found)*) $($rest)*)
    };

    ($finish:ident ($($found:ident)*)) => {
        $finish!(() $($found)*)
    };
}

// in:   nested_tuples_pat!(() a b c d e)
// out:  ((((a b) c) d) e)
//
// in:   nested_tuples_pat!(() a)
// out:  a
#[macro_export]
#[doc(hidden)]
macro_rules! nested_tuples_pat {
    (()) => {
        &()
    };

    (() $first:ident $($rest:ident)*) => {
        nested_tuples_pat!(($first) $($rest)*)
    };

    (($pat:pat) $first:ident $($rest:ident)*) => {
        nested_tuples_pat!((($pat, $first)) $($rest)*)
    };

    (($done:pat)) => {
        $done
    };
}

// in:   multi_zip_expr!(() a b c d e)
// out:  a.into_iter().zip(b).zip(c).zip(d).zip(e)
//
// in:   multi_zip_iter!(() a)
// out:  a
#[macro_export]
#[doc(hidden)]
macro_rules! multi_zip_expr {
    (()) => {
        &[]
    };

    (() $single:ident) => {
        $single
    };

    (() $first:ident $($rest:ident)*) => {
        multi_zip_expr!(($first.into_iter()) $($rest)*)
    };

    (($zips:expr) $first:ident $($rest:ident)*) => {
        multi_zip_expr!(($zips.zip($first)) $($rest)*)
    };

    (($done:expr)) => {
        $done
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! quote_each_token {
    ($tokens:ident $span:ident) => {};

    ($tokens:ident $span:ident # ! $($rest:tt)*) => {
        quote_each_token!($tokens $span #);
        quote_each_token!($tokens $span !);
        quote_each_token!($tokens $span $($rest)*);
    };

    ($tokens:ident $span:ident # ( $($inner:tt)* ) * $($rest:tt)*) => {
        for pounded_var_names!(nested_tuples_pat () $($inner)*)
        in pounded_var_names!(multi_zip_expr () $($inner)*) {
            quote_each_token!($tokens $span $($inner)*);
        }
        quote_each_token!($tokens $span $($rest)*);
    };

    ($tokens:ident $span:ident # ( $($inner:tt)* ) $sep:tt * $($rest:tt)*) => {
        for (_i, pounded_var_names!(nested_tuples_pat () $($inner)*))
        in pounded_var_names!(multi_zip_expr () $($inner)*).into_iter().enumerate() {
            if _i > 0 {
                quote_each_token!($tokens $span $sep);
            }
            quote_each_token!($tokens $span $($inner)*);
        }
        quote_each_token!($tokens $span $($rest)*);
    };

    ($tokens:ident $span:ident # [ $($inner:tt)* ] $($rest:tt)*) => {
        quote_each_token!($tokens $span #);
        $crate::__rt::append_kind(&mut $tokens,
            $span,
            $crate::__rt::TokenNode::Group(
                $crate::__rt::Delimiter::Bracket,
                quote_spanned!($span=> $($inner)*).into()
            ));
        quote_each_token!($tokens $span $($rest)*);
    };

    ($tokens:ident $span:ident # $first:ident $($rest:tt)*) => {
        $crate::ToTokens::to_tokens(&$first, &mut $tokens);
        quote_each_token!($tokens $span $($rest)*);
    };

    ($tokens:ident $span:ident ( $($first:tt)* ) $($rest:tt)*) => {
        $crate::__rt::append_kind(&mut $tokens,
            $span,
            $crate::__rt::TokenNode::Group(
                $crate::__rt::Delimiter::Parenthesis,
                quote_spanned!($span=> $($first)*).into()
            ));
        quote_each_token!($tokens $span $($rest)*);
    };

    ($tokens:ident $span:ident [ $($first:tt)* ] $($rest:tt)*) => {
        $crate::__rt::append_kind(&mut $tokens,
            $span,
            $crate::__rt::TokenNode::Group(
                $crate::__rt::Delimiter::Bracket,
                quote_spanned!($span=> $($first)*).into()
            ));
        quote_each_token!($tokens $span $($rest)*);
    };

    ($tokens:ident $span:ident { $($first:tt)* } $($rest:tt)*) => {
        $crate::__rt::append_kind(&mut $tokens,
            $span,
            $crate::__rt::TokenNode::Group(
                $crate::__rt::Delimiter::Brace,
                quote_spanned!($span=> $($first)*).into()
            ));
        quote_each_token!($tokens $span $($rest)*);
    };

    ($tokens:ident $span:ident $first:tt $($rest:tt)*) => {
        // TODO: this seems slow... special case some `:tt` arguments?
        $crate::__rt::parse(&mut $tokens, $span, stringify!($first));
        quote_each_token!($tokens $span $($rest)*);
    };
}
