use std::mem;

use self::WillOrMight::{Will,Might};
use self::BufferedParserState::{Beginning,Middle,End};
use self::MatchResult::{WillMatch, MightMatch, WontMatch, MatchedAll, MatchedSome};

// ----------- Types for matching -------------

// Borrowing encoding of paramaterized types from
// https://github.com/rust-lang/rfcs/blob/master/text/0195-associated-items.md#encoding-higher-kinded-types

pub trait RefType<'a> {
    type Ref;
}

pub type Ref<'a,T> where T: RefType<'a> = T::Ref;

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub enum MatchResult<T> {
    WillMatch,
    MightMatch,
    WontMatch,
    MatchedAll,
    MatchedSome(T,T),
}

pub trait Matcher<T> where T: 'static+for<'a> RefType<'a> {
    // If push returns MightMatch or WontMatch, it is side-effect-free
    fn push<'a>(&mut self, value: Ref<'a,T>) -> MatchResult<Ref<'a,T>>;
    // Resets the matcher state back to its initial state
    fn done(&mut self);
}

// ----------- Types for parsers ------------

pub trait Consumer<T> where T: 'static+for<'a> RefType<'a> {
    fn accept<'a>(&mut self, arg: Ref<'a,T>);
}

pub trait Parser<S,T> where S: 'static+for<'a> RefType<'a>, T: 'static+for<'a> RefType<'a> {
    fn push<'a>(&mut self, value: Ref<'a,S>, downstream: &mut Consumer<T>) -> MatchResult<Ref<'a,S>>;
    fn done(&mut self, downstream: &mut Consumer<T>);
}

pub trait BufferableMatcher<S,T> where S: 'static+for<'a> RefType<'a>, T: Parser<S,S> {
    fn buffer(self) -> T;
}

// ----------- Matching strings -------------

pub struct StrRef;

impl<'a> RefType<'a> for StrRef {
    type Ref = &'a str;
}

enum WillOrMight {
    Will,
    Might,
}

enum BufferedParserState {
    Beginning,
    Middle(String,WillOrMight),
    End,
}

pub struct BufferedParser<S> {
    matcher: S,
    state: BufferedParserState,
}

impl<S> Parser<StrRef,StrRef> for BufferedParser<S> where S: Matcher<StrRef> {
    fn push<'a>(&mut self, string: &'a str, downstream: &mut Consumer<StrRef>) -> MatchResult<&'a str> {
        match mem::replace(&mut self.state, End) {
            Beginning => {
                let result = self.matcher.push(string);
                match result {
                    WillMatch               => self.state = Middle(String::from(string), Will),
                    MightMatch              => self.state = Middle(String::from(string), Might),
                    WontMatch               => (),
                    MatchedAll              => downstream.accept(string),
                    MatchedSome(matched, _) => downstream.accept(matched),
                }
                result
            },
            Middle(mut buffer, _) => {
                let result = self.matcher.push(string);
                match result {
                    WillMatch               => { buffer.push_str(string); self.state = Middle(buffer, Will); },
                    MightMatch              => { buffer.push_str(string); self.state = Middle(buffer, Might); },
                    WontMatch               => (),
                    MatchedAll              => { buffer.push_str(string); downstream.accept(&*buffer); },
                    MatchedSome(matched, _) => { buffer.push_str(matched); downstream.accept(&*buffer); },
                }
                result
            }
            End => MatchedSome("",string),
        }
    }
    fn done(&mut self, downstream: &mut Consumer<StrRef>) {
        match mem::replace(&mut self.state, Beginning) {
            Middle(buffer, Will) => downstream.accept(&*buffer),
            _                    => (),
        }
    }
}

impl<S> BufferableMatcher<StrRef,BufferedParser<S>> for S where S: Matcher<StrRef> {
    fn buffer(self) -> BufferedParser<S> {
        BufferedParser{ matcher: self, state: Beginning }
    }
}
