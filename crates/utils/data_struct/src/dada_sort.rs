/// Quick sort a slice with a custom comparison function
///
/// # Arguments
/// * `arr` - The mutable slice to be sorted
/// * `inverse` - Sort direction: true for descending, false for ascending
/// * `compare` - Comparison function that returns -1, 0, or 1 indicating the relative order of two elements
pub fn quick_sort_with_cmp<T, F>(arr: &mut [T], inverse: bool, compare: F)
where
    F: Fn(&T, &T) -> i32,
{
    quick_sort_with_cmp_helper(arr, inverse, &compare);
}

/// Quick sort for types that implement the PartialOrd trait
///
/// # Arguments
/// * `arr` - The mutable slice to be sorted
/// * `inverse` - Sort direction: true for descending, false for ascending
pub fn quick_sort<T: PartialOrd>(arr: &mut [T], inverse: bool) {
    quick_sort_with_cmp(arr, inverse, |a, b| {
        if a < b {
            -1
        } else if a > b {
            1
        } else {
            0
        }
    });
}

fn quick_sort_with_cmp_helper<T, F>(arr: &mut [T], inverse: bool, compare: &F)
where
    F: Fn(&T, &T) -> i32,
{
    if arr.len() <= 1 {
        return;
    }

    let pivot_index = partition_with_cmp(arr, inverse, compare);
    let (left, right) = arr.split_at_mut(pivot_index);

    quick_sort_with_cmp_helper(left, inverse, compare);
    quick_sort_with_cmp_helper(&mut right[1..], inverse, compare);
}

fn partition_with_cmp<T, F>(arr: &mut [T], inverse: bool, compare: &F) -> usize
where
    F: Fn(&T, &T) -> i32,
{
    let len = arr.len();
    let pivot_index = len / 2;

    arr.swap(pivot_index, len - 1);

    let mut i = 0;
    for j in 0..len - 1 {
        let cmp_result = compare(&arr[j], &arr[len - 1]);
        let should_swap = if inverse {
            cmp_result > 0
        } else {
            cmp_result < 0
        };

        if should_swap {
            arr.swap(i, j);
            i += 1;
        }
    }

    arr.swap(i, len - 1);
    i
}

#[cfg(test)]
pub mod sort_test {
    use crate::dada_sort::{quick_sort, quick_sort_with_cmp};

    #[test]
    fn test_quick_sort_ascending() {
        let mut arr = [3, 1, 4, 1, 5, 9, 2, 6];
        quick_sort(&mut arr, false);
        assert_eq!(arr, [1, 1, 2, 3, 4, 5, 6, 9]);
    }

    #[test]
    fn test_quick_sort_descending() {
        let mut arr = [3, 1, 4, 1, 5, 9, 2, 6];
        quick_sort(&mut arr, true);
        assert_eq!(arr, [9, 6, 5, 4, 3, 2, 1, 1]);
    }

    #[test]
    fn test_quick_sort_single() {
        let mut arr = [42];
        quick_sort(&mut arr, false);
        assert_eq!(arr, [42]);
    }

    #[test]
    fn test_quick_sort_already_sorted() {
        let mut arr = [1, 2, 3, 4, 5];
        quick_sort(&mut arr, false);
        assert_eq!(arr, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_quick_sort_with_cmp_by_count() {
        #[derive(Debug, PartialEq)]
        struct WordCount {
            word: String,
            count: usize,
        }

        let mut words = vec![
            WordCount {
                word: "apple".to_string(),
                count: 3,
            },
            WordCount {
                word: "banana".to_string(),
                count: 1,
            },
            WordCount {
                word: "cherry".to_string(),
                count: 5,
            },
            WordCount {
                word: "date".to_string(),
                count: 2,
            },
        ];

        quick_sort_with_cmp(&mut words, false, |a, b| {
            if a.count < b.count {
                -1
            } else if a.count > b.count {
                1
            } else {
                0
            }
        });

        assert_eq!(
            words,
            vec![
                WordCount {
                    word: "banana".to_string(),
                    count: 1
                },
                WordCount {
                    word: "date".to_string(),
                    count: 2
                },
                WordCount {
                    word: "apple".to_string(),
                    count: 3
                },
                WordCount {
                    word: "cherry".to_string(),
                    count: 5
                },
            ]
        );

        quick_sort_with_cmp(&mut words, true, |a, b| {
            if a.count < b.count {
                -1
            } else if a.count > b.count {
                1
            } else {
                0
            }
        });

        assert_eq!(
            words,
            vec![
                WordCount {
                    word: "cherry".to_string(),
                    count: 5
                },
                WordCount {
                    word: "apple".to_string(),
                    count: 3
                },
                WordCount {
                    word: "date".to_string(),
                    count: 2
                },
                WordCount {
                    word: "banana".to_string(),
                    count: 1
                },
            ]
        );
    }

    #[test]
    fn test_quick_sort_with_cmp_by_first_letter() {
        let mut words = vec!["zebra", "apple", "banana", "cherry", "date"];

        quick_sort_with_cmp(&mut words, false, |a, b| {
            let a_first = a.chars().next().unwrap();
            let b_first = b.chars().next().unwrap();

            if a_first < b_first {
                -1
            } else if a_first > b_first {
                1
            } else {
                0
            }
        });

        assert_eq!(words, vec!["apple", "banana", "cherry", "date", "zebra"]);

        quick_sort_with_cmp(&mut words, true, |a, b| {
            let a_first = a.chars().next().unwrap();
            let b_first = b.chars().next().unwrap();

            if a_first < b_first {
                -1
            } else if a_first > b_first {
                1
            } else {
                0
            }
        });

        assert_eq!(words, vec!["zebra", "date", "cherry", "banana", "apple"]);
    }
}
