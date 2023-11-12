use crate::world::{BlockCoord, BlockVolume};

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

    //will iterate over the max bounds of volume
    pub fn from_volume_inclusive(volume: BlockVolume) -> impl Iterator {
        Self::from_volume_exclusive(BlockVolume::new(volume.min_corner, volume.max_corner + BlockCoord::new(1,1,1)))
    }

    //will iterate over the max bounds of volume
    pub fn from_volume_exclusive(volume: BlockVolume) -> impl Iterator {
        let size = volume.max_corner - volume.min_corner;
        Self {
            x_len: size.x,
            y_len: size.y,
            z_len: size.z,
            x_i: 0,
            y_i: 0,
            z_i: 0,
            done: size.x <= 0 || size.y <= 0 || size.z <= 0,
        }
        .map(move |offset| volume.min_corner + offset)
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
fn test_volume_iterator() {
    let it = BlockVolumeIterator::new(5, 6, 7);
    let mut count = 0;
    for coord in it {
        count += 1;
        assert!(coord.x < 5);
        assert!(coord.y < 6);
        assert!(coord.z < 7);
    }
    assert_eq!(count, 5 * 6 * 7);
}

#[test]
fn test_volume_iterator_zero() {
    let it = BlockVolumeIterator::new(0, 0, 0);
    let mut count = 0;
    for _ in it {
        count += 1;
    }
    assert_eq!(count, 0);
}

#[test]
fn test_volume_iterator_one() {
    let it = BlockVolumeIterator::new(1, 1, 1);
    let mut count = 0;
    for _ in it {
        count += 1;
    }
    assert_eq!(count, 1);
}
