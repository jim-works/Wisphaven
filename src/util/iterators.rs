use crate::world::BlockCoord;

pub struct BlockVolumeIterator {
    x_len: i32,
    y_len: i32,
    z_len: i32,
    x_i: i32,
    y_i: i32,
    z_i: i32,
    done: bool, //this is ugly but i'm tired
}

impl BlockVolumeIterator {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            x_len: x as i32,
            y_len: y as i32,
            z_len: z as i32,
            x_i: 0,
            y_i: 0,
            z_i: 0,
            done: x == 0 || y == 0 || z == 0,
        }
    }
}

impl Iterator for BlockVolumeIterator {
    type Item = BlockCoord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let ret = Some(BlockCoord::new(self.x_i, self.y_i, self.z_i));
        self.x_i += 1;
        if self.x_i >= self.x_len {
            self.y_i += 1;
            self.x_i = 0;
        }
        if self.y_i >= self.y_len {
            self.z_i += 1;
            self.y_i = 0;
        }
        if self.z_i >= self.z_len {
            self.done = true;
        }
        return ret;
    }
}

#[test]
fn test_volume_iterator()
{
    let it = BlockVolumeIterator::new(5,6,7);
    let mut count = 0;
    for coord in it {
        count += 1;
        assert!(coord.x < 5);
        assert!(coord.y < 6);
        assert!(coord.z < 7);
    }
    assert_eq!(count, 5*6*7);
}

#[test]
fn test_volume_iterator_zero()
{
    let it = BlockVolumeIterator::new(0,0,0);
    let mut count = 0;
    for _ in it {
        count += 1;
    }
    assert_eq!(count, 0);
}

#[test]
fn test_volume_iterator_one()
{
    let it = BlockVolumeIterator::new(1,1,1);
    let mut count = 0;
    for _ in it {
        count += 1;
    }
    assert_eq!(count, 1);
}