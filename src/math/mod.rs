use std::ops::{Add, AddAssign, BitXor, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign};

pub mod numeric_traits {
    pub trait Sqrt {
        fn sqrt(self) -> Self;
    }

    impl Sqrt for f32 {
        fn sqrt(self) -> Self {
            self.sqrt()
        }
    }

    impl Sqrt for f64 {
        fn sqrt(self) -> Self {
            self.sqrt()
        }
    }
}

macro_rules! impl_vecn_binary_operator {
    ($op_name: ident, $op_fn_name: ident, $struct_name: ident, $($x: ident),*) => {
        impl<T: $op_name<Output = T>> $op_name<$struct_name<T>> for $struct_name<T> {
            type Output = $struct_name<T>;

            fn $op_fn_name(self, rhs: $struct_name<T>) -> Self::Output {
                Self::Output {
                    $( $x: $op_name::<T>::$op_fn_name(self.$x, rhs.$x), )*
                }
            }
        }

        impl<T: Clone + $op_name<Output = T>> $op_name<T> for $struct_name<T> {
            type Output = $struct_name<T>;

            fn $op_fn_name(self, rhs: T) -> Self::Output {
                Self::Output {
                    $( $x: $op_name::<T>::$op_fn_name(self.$x, rhs.clone()), )*
                }
            }
        }
    }
}

macro_rules! impl_vecn_assignment_operator {
    ($op_name: ident, $op_fn_name: ident, $struct_name: ident, $($x: ident),*) => {
        impl<T: $op_name> $op_name<$struct_name<T>> for $struct_name<T> {
            fn $op_fn_name(&mut self, rhs: $struct_name<T>) {
                $( $op_name::<T>::$op_fn_name(&mut self.$x, rhs.$x); )*
            }
        }

        impl<T: Clone + $op_name> $op_name<T> for $struct_name<T> {
            fn $op_fn_name(&mut self, rhs: T) {
                $( $op_name::<T>::$op_fn_name(&mut self.$x, rhs.clone()); )*
            }
        }
    }
}

macro_rules! impl_vecn_unary_operator {
    ($op_name: ident, $op_fn_name: ident, $struct_name: ident, $($x: ident),*) => {
        impl<T: $op_name<Output = T>> $op_name for $struct_name<T> {
            type Output = $struct_name<T>;

            fn $op_fn_name(self) -> Self::Output {
                Self::Output {
                    $( $x: $op_name::$op_fn_name(self.$x), )*
                }
            }
        }
    }
}

macro_rules! impl_vecn {
    ($struct_name: ident, $($x: ident),*) => {
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $struct_name<T> {
            $( pub $x : T, )*
        }

        impl<T> $struct_name<T> {
            pub fn new($($x: T,)*) -> Self {
                Self { $($x,)* }
            }
        }

        impl_vecn_binary_operator!(Add, add, $struct_name, $($x),*);
        impl_vecn_binary_operator!(Sub, sub, $struct_name, $($x),*);
        impl_vecn_binary_operator!(Mul, mul, $struct_name, $($x),*);
        impl_vecn_binary_operator!(Div, div, $struct_name, $($x),*);

        impl_vecn_unary_operator!(Neg, neg, $struct_name, $($x),*);

        impl_vecn_assignment_operator!(AddAssign, add_assign, $struct_name, $($x),*);
        impl_vecn_assignment_operator!(SubAssign, sub_assign, $struct_name, $($x),*);
        impl_vecn_assignment_operator!(MulAssign, mul_assign, $struct_name, $($x),*);
        impl_vecn_assignment_operator!(DivAssign, div_assign, $struct_name, $($x),*);
    }
}

impl_vecn!(Vec2, x, y);
impl_vecn!(Vec3, x, y, z);
impl_vecn!(Vec4, x, y, z, w);

#[derive(Copy, Clone, Default)]
pub struct Ext2<T> {
    pub w: T,
    pub h: T,
}

impl<T> Ext2<T> {
    pub fn new(w: T, h: T) -> Self {
        Self { w, h }
    }
}

pub type Ext2u = Ext2<u32>;
pub type Ext2f = Ext2<f32>;
pub type Vec2f = Vec2<f32>;
pub type Vec3f = Vec3<f32>;

impl Rem for Vec3f {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.y * rhs.z - self.z * rhs.y,
            y: self.z * rhs.x - self.x * rhs.z,
            z: self.x * rhs.y - self.y * rhs.x,
        }
    }
}

impl RemAssign for Vec3f {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl BitXor for Vec3f {
    type Output = f32;
    fn bitxor(self, rhs: Self) -> Self::Output {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
}

impl BitXor for Vec2f {
    type Output = f32;
    fn bitxor(self, rhs: Self) -> Self::Output {
        self.x * rhs.x + self.y * rhs.y
    }
}

impl Vec3f {
    #[inline]
    pub fn length2(&self) -> f32 {
        *self ^ *self
    }

    #[inline]
    pub fn length(&self) -> f32 {
        self.length2().sqrt()
    }

    #[inline]
    pub fn normalized(&self) -> Self {
        *self / self.length()
    }

    #[inline]
    pub fn normalize(&mut self) {
        *self /= self.length();
    }
}

impl Vec2f {
    #[inline]
    pub fn length2(&self) -> f32 {
        *self ^ *self
    }

    #[inline]
    pub fn length(&self) -> f32 {
        self.length2().sqrt()
    }

    #[inline]
    pub fn normalized(&self) -> Self {
        *self / self.length()
    }

    #[inline]
    pub fn normalize(&mut self) {
        *self /= self.length();
    }
}