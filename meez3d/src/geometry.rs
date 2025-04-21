use std::ops;

use num_traits::Zero;

// Points

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point<T> {
    #[inline]
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T> Zero for Point<T>
where
    T: Zero,
{
    #[inline]
    fn zero() -> Self {
        Self::new(T::zero(), T::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.x.is_zero() && self.y.is_zero()
    }

    #[inline]
    fn set_zero(&mut self) {
        self.x = T::zero();
        self.y = T::zero();
    }
}

impl<T> From<(T, T)> for Point<T> {
    #[inline]
    fn from(value: (T, T)) -> Self {
        Point::new(value.0, value.1)
    }
}

impl<T> ops::Add<Point<T>> for Point<T>
where
    T: ops::Add<T, Output = T>,
{
    type Output = Point<T>;

    #[inline]
    fn add(self, rhs: Point<T>) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T> ops::AddAssign<Point<T>> for Point<T>
where
    T: ops::AddAssign<T>,
{
    #[inline]
    fn add_assign(&mut self, rhs: Point<T>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T> ops::Sub<Point<T>> for Point<T>
where
    T: ops::Sub<T, Output = T>,
{
    type Output = Point<T>;

    #[inline]
    fn sub(self, rhs: Point<T>) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<T> ops::SubAssign<Point<T>> for Point<T>
where
    T: ops::SubAssign<T>,
{
    #[inline]
    fn sub_assign(&mut self, rhs: Point<T>) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T, U> ops::Mul<U> for Point<T>
where
    T: ops::Mul<U, Output = T>,
    U: Copy,
{
    type Output = Point<T>;

    #[inline]
    fn mul(self, rhs: U) -> Self::Output {
        Point::new(self.x * rhs, self.y * rhs)
    }
}

// Rect

#[derive(Debug, Clone, Copy)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}

impl<T> Rect<T>
where
    T: ops::Add<T, Output = T> + Copy + PartialOrd,
{
    #[inline]
    pub fn top(&self) -> T {
        self.y
    }
    #[inline]
    pub fn left(&self) -> T {
        self.x
    }
    #[inline]
    pub fn right(&self) -> T {
        self.x + self.w
    }
    #[inline]
    pub fn bottom(&self) -> T {
        self.y + self.h
    }
    #[inline]
    pub fn top_left(&self) -> Point<T> {
        Point::new(self.x, self.y)
    }

    pub fn intersects(&self, other: Rect<T>) -> bool {
        self.right() >= other.left()
            && self.left() <= other.right()
            && self.bottom() >= other.top()
            && self.top() <= other.bottom()
    }

    pub fn contains(&self, point: Point<T>) -> bool {
        point.x >= self.left()
            && point.x <= self.right()
            && point.y >= self.top()
            && point.y <= self.bottom()
    }
}

impl<T> ops::Add<Point<T>> for Rect<T>
where
    T: ops::Add<T, Output = T>,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: Point<T>) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            w: self.w,
            h: self.h,
        }
    }
}

impl<T> ops::AddAssign<Point<T>> for Rect<T>
where
    T: ops::AddAssign<T>,
{
    #[inline]
    fn add_assign(&mut self, rhs: Point<T>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_getters() {
        let r = Rect {
            x: 10,
            y: 20,
            w: 3,
            h: 4,
        };
        assert_eq!(r.x, 10);
        assert_eq!(r.y, 20);
        assert_eq!(r.w, 3);
        assert_eq!(r.h, 4);
        assert_eq!(r.left(), 10);
        assert_eq!(r.top(), 20);
        assert_eq!(r.right(), 13);
        assert_eq!(r.bottom(), 24);
    }

    #[test]
    fn rect_add_point() {
        let r = Rect {
            x: 10,
            y: 20,
            w: 3,
            h: 4,
        };
        let p = Point::new(100, 200);
        let r = r + p;
        assert_eq!(r.x, 110);
        assert_eq!(r.y, 220);
        assert_eq!(r.w, 3);
        assert_eq!(r.h, 4);
        assert_eq!(r.left(), 110);
        assert_eq!(r.top(), 220);
        assert_eq!(r.right(), 113);
        assert_eq!(r.bottom(), 224);
    }
}
