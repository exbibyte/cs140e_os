mod align_util {
    use allocator::util::{align_up, align_down};

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0, 2), 0);
        assert_eq!(align_down(0, 8), 0);
        assert_eq!(align_down(0, 1 << 5), 0);

        assert_eq!(align_down(1 << 10, 1 << 10), 1 << 10);
        assert_eq!(align_down(1 << 20, 1 << 10), 1 << 20);
        assert_eq!(align_down(1 << 23, 1 << 4), 1 << 23);

        assert_eq!(align_down(1, 1 << 4), 0);
        assert_eq!(align_down(10, 1 << 4), 0);

        assert_eq!(align_down(0xFFFF, 1 << 2), 0xFFFC);
        assert_eq!(align_down(0xFFFF, 1 << 3), 0xFFF8);
        assert_eq!(align_down(0xFFFF, 1 << 4), 0xFFF0);
        assert_eq!(align_down(0xFFFF, 1 << 5), 0xFFE0);
        assert_eq!(align_down(0xAFFFF, 1 << 8), 0xAFF00);
        assert_eq!(align_down(0xAFFFF, 1 << 12), 0xAF000);
        assert_eq!(align_down(0xAFFFF, 1 << 16), 0xA0000);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 2), 0);
        assert_eq!(align_up(0, 8), 0);
        assert_eq!(align_up(0, 1 << 5), 0);

        assert_eq!(align_up(1 << 10, 1 << 10), 1 << 10);
        assert_eq!(align_up(1 << 20, 1 << 10), 1 << 20);
        assert_eq!(align_up(1 << 23, 1 << 4), 1 << 23);

        assert_eq!(align_up(1, 1 << 4), 1 << 4);
        assert_eq!(align_up(10, 1 << 4), 1 << 4);

        assert_eq!(align_up(0xFFFF, 1 << 2), 0x10000);
        assert_eq!(align_up(0xFFFF, 1 << 3), 0x10000);
        assert_eq!(align_up(0xFFFF, 1 << 4), 0x10000);
        assert_eq!(align_up(0xAFFFF, 1 << 12), 0xB0000);

        assert_eq!(align_up(0xABCDAB, 1 << 2), 0xABCDAC);
        assert_eq!(align_up(0xABCDAB, 1 << 4), 0xABCDB0);
        assert_eq!(align_up(0xABCDAB, 1 << 8), 0xABCE00);
        assert_eq!(align_up(0xABCDAB, 1 << 12), 0xABD000);
        assert_eq!(align_up(0xABCDAB, 1 << 16), 0xAC0000);
    }

    #[test] #[should_panic] fn test_panics_1() { align_down(0xFFFF0000, 7); }
    #[test] #[should_panic] fn test_panics_2() { align_down(0xFFFF0000, 123); }
    #[test] #[should_panic] fn test_panics_3() { align_up(0xFFFF0000, 7); }
    #[test] #[should_panic] fn test_panics_4() { align_up(0xFFFF0000, 456); }
}

#[path = ""]
mod allocator {
    #[allow(dead_code)] mod bump;
    #[allow(dead_code)] mod bin;

    use alloc::allocator::{AllocErr, Layout};
    use alloc::raw_vec::RawVec;

    macro test_allocators {
        (@$kind:ident, $name:ident, $mem:expr, |$info:pat| $block:expr) => {
            #[test]
            fn $name() {
                let mem: RawVec<u8> = RawVec::with_capacity($mem);
                let start = mem.ptr() as usize;
                let end = start + $mem;

                let allocator = $kind::Allocator::new(start, end);
                let $info = (start, end, allocator);
                $block
            }
        },

        ($bin:ident, $bump:ident, $mem:expr, |$info:pat| $block:expr) => (
            test_allocators!(@bin, $bin, $mem, |$info| $block);
            test_allocators!(@bump, $bump, $mem, |$info| $block);
        )
    }

    macro layout($size:expr, $align:expr) {
        Layout::from_size_align($size, $align).unwrap()
    }

    macro test_layouts($layouts:expr, $start:expr, $end:expr, $a:expr) {
        let (layouts, start, end, mut a) = ($layouts, $start, $end, $a);

        let mut pointers: Vec<(usize, Layout)> = vec![];
        for layout in &layouts {
            let ptr = a.alloc(layout.clone()).unwrap() as usize;
            pointers.push((ptr, layout.clone()));
        }

        // Check that we have allocations after 'start' and before 'end'.
        for &(ptr, ref layout) in &pointers {
            assert!(ptr >= start, "allocated {:x} after start ({:x})", ptr, start);
            assert!(ptr + layout.size() <= end,
                "{:x} + {:x} exceeds the bounds of {:x}", ptr, layout.size(), end);
        }

        // Check that we have non-overlapping allocations.
        pointers.sort_by_key(|&(ptr, _)| ptr);
        for window in pointers.windows(2) {
            let (&(ptr_a, ref layout_a), &(ptr_b, _)) = (&window[0], &window[1]);
            assert!(ptr_b - ptr_a >= layout_a.size(),
                "memory region {:x} - {:x} does not fit {}", ptr_a, ptr_b, layout_a.size());
        }

        // Check alignment.
        for &(ptr, ref layout) in &pointers {
            assert!(ptr % layout.align() == 0,
                "{:x} is not aligned to {}", ptr, layout.align());
        }
    }

    test_allocators!(bin_exhausted, bump_exhausted, 128, |(_, _, mut a)| {
        let e = a.alloc(layout!(1024, 128)).unwrap_err();
        assert_eq!(e, AllocErr::Exhausted { request: layout!(1024, 128) })
    });

    test_allocators!(bin_alloc, bump_alloc, 8 * (1 << 20), |(start, end, a)| {
        let layouts = [
            layout!(16, 16),
            layout!(16, 128),
            layout!(16, 256),
            layout!(4, 256),
            layout!(1024, 16),
            layout!(1024, 4),
            layout!(1024, 128),
            layout!(2048, 8),
            layout!(2049, 8),
            layout!(2050, 8),
            layout!(4095, 4),
            layout!(4096, 4),
            layout!(4096, 4),
            layout!(4096, 4096),
            layout!(16, 4096),
            layout!(8192, 4096),
            layout!(8192, 8),
            layout!(8192, 8),
        ];

        // Test a few specially chosen layouts.
        test_layouts!(layouts, start, end, a);
    });

    test_allocators!(bin_alloc_2, bump_alloc_2, 16 * (1 << 20), |(start, end, a)| {
        let mut layouts = vec![];
        for i in 1..1024 {
            layouts.push(layout!(i * 8, 16));
        }

        // Ensure ~contiguous allocations are properly handled.
        test_layouts!(layouts, start, end, a);
    });

    test_allocators!(bin_dealloc_s, bump_dealloc_s, 4096, |(_, _, mut a)| {
        let layouts = [
            layout!(16, 16),
            layout!(16, 128),
            layout!(16, 256),
        ];

        let mut pointers: Vec<(usize, Layout)> = vec![];
        for layout in &layouts {
            let ptr = a.alloc(layout.clone()).unwrap() as usize;
            pointers.push((ptr, layout.clone()));
        }

        // Just check that deallocation doesn't panic.
        for (ptr, layout) in pointers {
            a.dealloc(ptr as *mut u8, layout);
        }
    });

    test_allocators!(@bin, bin_dealloc, 8192, |(_, _, mut a)| {
        let layouts = [
            layout!(3072, 16),
            layout!(512, 32),
        ];

        // ensure we can reuse freed memory. also tests that the allocator has
        // resonable internal fragmentation
        for _ in 0..1000 {
            let mut ptrs = vec![];
            for layout in &layouts {
                ptrs.push(a.alloc(layout.clone()).expect("allocation") as usize);
            }

            for (layout, ptr) in layouts.iter().zip(ptrs.into_iter()) {
                a.dealloc(ptr as *mut u8, layout.clone());
            }
        }
    });
}