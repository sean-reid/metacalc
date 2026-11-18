use crate::raw;
use mcpack::library::{CheckError, ExportError, Kernel, Limits, check, export};

#[test]
fn the_kernel_and_the_exporter_accept_and_refuse_the_same_records() {
    for case in raw::record_cases() {
        let mut kernel = Kernel::new(&case.bytes, Limits::UNBOUNDED).expect(case.name);
        assert_eq!(kernel.verify(0), case.expected, "{}", case.name);
        let exported = export(&case.bytes);
        match case.expected {
            Ok(()) => {
                assert_eq!(check(&case.bytes), Ok(1), "{}", case.name);
                assert!(exported.is_ok(), "{}", case.name);
            }
            Err(e) => {
                let failed = || CheckError::Assertions(vec![("ax-1".into(), e)]);
                assert_eq!(check(&case.bytes), Err(failed()), "{}", case.name);
                assert_eq!(exported, Err(ExportError::Check(failed())), "{}", case.name);
            }
        }
    }
}
