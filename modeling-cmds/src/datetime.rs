use kittycad_execution_plan_traits::{MemoryError, NumericPrimitive, Primitive, Value};

/// A wrapper for chrono types, since we need to impl Value for them.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DateTimeLocal {
    value: chrono::DateTime<chrono::Local>,
}

impl Value for DateTimeLocal {
    fn into_parts(self) -> Vec<Primitive> {
        vec![Primitive::NumericValue(NumericPrimitive::Integer(
            self.value.timestamp_nanos_opt().unwrap(),
        ))]
    }

    /// Read the value from memory.
    fn from_parts<I>(values: &mut I) -> Result<(Self, usize), MemoryError>
    where
        I: Iterator<Item = Option<Primitive>>,
    {
        let Some(maybe_datetime) = values.next() else {
            return Err(MemoryError::MemoryBadAccess);
        };

        match maybe_datetime {
            None => Err(MemoryError::MemoryBadAccess),
            Some(Primitive::NumericValue(NumericPrimitive::Integer(timestamp_nanos))) => Ok((
                DateTimeLocal {
                    value: chrono::DateTime::from_timestamp_nanos(timestamp_nanos).into(),
                },
                1,
            )),
            Some(o) => Err(MemoryError::MemoryWrongType {
                expected: "i64 numeric timestamp expected",
                actual: format!("{:?}", o),
            }),
        }
    }
}

#[test]
fn datetime_into_from_values() {
    let a = DateTimeLocal {
        value: chrono::Local::now(),
    };
    let Ok((b, _)) = DateTimeLocal::from_parts(&mut a.clone().into_parts().into_iter().map(Some)) else {
        unreachable!();
    };

    assert_eq!(a, b);
}
