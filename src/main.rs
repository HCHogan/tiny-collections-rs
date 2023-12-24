#![allow(unused)]
use tiny_collections_rs::btreemap::map::BTreeMap;

fn main() {
        let mut bt = BTreeMap::new();
        (0..100).for_each(|i| {
            bt.insert(i, i);
        });

        (0..100).for_each(|i| {
            assert_eq!(Some(&i), bt.find(&i));
        });
}

fn test() {
    println!("Hello, world!");
    let mut v = vec![1i32, 2, 3, 4];
    let x = v.get(2);
    v.push(6);
    let mut v = [1i32, 2, 3, 4, 5];
    let x = v.first(); // Sure
    let y = v.get(1); // Mhmm
    let z = v.get_mut(2); // Should be fine..?
}

fn test1() {
    let v = vec![1, 2, 3];
    check_exact_size_iter(v);
}

fn check_exact_size_iter<T>(c: T) -> usize
where
    T: IntoIterator,
    <T as IntoIterator>::Item: std::fmt::Debug,
    <T as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator,
{
    for e in c {
        println!("{:?}", e);
    }
    0
}

// fn find_min<T: Ord>(data: Vec<T>) -> Option<T> {
fn find_min<T>(data: T) -> Option<<T as IntoIterator>::Item>
where
    T: IntoIterator,
    <T as IntoIterator>::Item: Ord,
{
    let mut it = data.into_iter();
    let mut min = match it.next() {
        Some(elem) => elem,
        None => return None,
    };
    for elem in it {
        if elem < min {
            min = elem;
        }
    }
    Some(min)
}
