// ir/src/hir/id.rs

macro_rules! define_id {
    ($name:ident) => {
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);

        impl $name {
            pub const fn new(index: usize) -> Self {
                Self(index as u32)
            }

            pub const fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

define_id!(HirId);
define_id!(BodyId);
define_id!(ExprId);
define_id!(MatchId);
define_id!(MatchArmId);
define_id!(ConditionId);
