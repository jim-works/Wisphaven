//from https://stackoverflow.com/questions/43921436/extend-iterator-with-a-mean-method#43926007
pub trait MeanExt: Iterator {
    fn mean<M>(self) -> M
    where
        M: Mean<Self::Item>,
        Self: Sized,
    {
        M::mean(self)
    }
}

impl<I: Iterator> MeanExt for I {}

pub trait Mean<A = Self> {
    fn mean<I>(iter: I) -> Self
    where
        I: Iterator<Item = A>;
}

impl Mean for f32 {
    fn mean<I>(iter: I) -> Self
    where
        I: Iterator<Item = f32>,
    {
        let mut sum = 0.0;
        let mut count: usize = 0;

        for v in iter {
            sum += v;
            count += 1;
        }

        if count > 0 {
            sum / (count as f32)
        } else {
            0.0
        }
    }
}

impl<'a> Mean<&'a f32> for f32 {
    fn mean<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a f32>,
    {
        iter.copied().mean()
    }
}