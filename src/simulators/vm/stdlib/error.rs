use crate::definitions::Word;
use crate::simulators::vm::VMError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdlibError {
    // general/internal errors
    IncorrectNumberOfArgs,
    CallingNonExistendFunction,
    ContinuingFinishedFunction,

    // this needs to be a box, because VMError and StdlibError have a circular relationship
    VMError(Box<VMError>),

    // Sys.vm errors
    SysError(Word), // returned by the Sys.err function
    SysWaitNegativeDuration,

    // Math.vm errors
    MathDivideByZero,
    MathNegativeSqrt,

    // Memory.vm errors
    MemoryAllocNonPositiveSize,
    MemoryHeapOverflow,

    // Array.vm errors
    ArrayNewNonPositiveSize,

    // Screen.vm errors
    ScreenBlockedColorMutex,
    ScreenIllegalCoords,

    // String.vm errors
    StringNewNegativeLength,
    StringCharAtIllegalIndex,
    StringSetCharAtIllegalIndex,
    StringAppendCharFull,
    StringEraseLastCharEmtpy,
    StringSetIntInsufficientCapacity,

    // Output.vm errors
    OutputBlockedAddressMutex,
    OutputBlockedFirstInWordMutex,
    OutputBlockedWordInLineMutex,
    OutputMoveCursorIllegalPosition,
}

impl From<VMError> for StdlibError {
    fn from(e: VMError) -> Self {
        Self::VMError(Box::new(e))
    }
}

const VM_ERRORS: [&str; 18] = [
    "",
    "Duration must be positive",
    "Array size must be positive",
    "Division by zero",
    "Cannot compute square root of a negative number",
    "Allocated memory size must be positive",
    "Heap overflow",
    "Illegal pixel coordinates",
    "Illegal line coordinates",
    "Illegal rectangle coordinates",
    "Illegal center coordinates",
    "Illegal radius",
    "Maximum length must be non-negative",
    "String index out of bounds",
    "String is full",
    "String is empty",
    "Insufficient string capacity",
    "Illegal cursor location",
];

impl fmt::Display for StdlibError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IncorrectNumberOfArgs => write!(f, "Incorrect number of arguments"),
            Self::CallingNonExistendFunction => {
                write!(f, "Trying to call non existing stdlib function")
            }
            Self::ContinuingFinishedFunction => write!(f, "Trying to continue finished function"),
            Self::VMError(vm_error) => write!(f, "{}", vm_error),
            Self::SysError(error) => {
                if (1..18).contains(error) {
                    write!(f, "{}", VM_ERRORS[*error as usize])
                } else {
                    write!(f, "Unknown error code: {}", error)
                }
            }
            Self::SysWaitNegativeDuration => write!(f, "{}", VM_ERRORS[1]),
            Self::MathDivideByZero => write!(f, "{}", VM_ERRORS[3]),
            Self::MathNegativeSqrt => write!(f, "{}", VM_ERRORS[4]),
            Self::MemoryAllocNonPositiveSize => write!(f, "{}", VM_ERRORS[5]),
            Self::MemoryHeapOverflow => write!(f, "{}", VM_ERRORS[6]),
            Self::ArrayNewNonPositiveSize => write!(f, "{}", VM_ERRORS[2]),
            Self::ScreenBlockedColorMutex => write!(
                f,
                "Blocked color Mutex in Screen, this should be impossible"
            ),
            Self::ScreenIllegalCoords => write!(f, "{}", VM_ERRORS[7]),
            Self::StringNewNegativeLength => write!(f, "{}", VM_ERRORS[12]),
            Self::StringCharAtIllegalIndex => write!(f, "{}", VM_ERRORS[13]),
            Self::StringSetCharAtIllegalIndex => write!(f, "{}", VM_ERRORS[13]),
            Self::StringAppendCharFull => write!(f, "{}", VM_ERRORS[14]),
            Self::StringEraseLastCharEmtpy => write!(f, "{}", VM_ERRORS[15]),
            Self::StringSetIntInsufficientCapacity => write!(f, "{}", VM_ERRORS[16]),
            Self::OutputBlockedAddressMutex => write!(
                f,
                "Blocked address mutex in Screen, this should be impossible"
            ),
            Self::OutputBlockedFirstInWordMutex => write!(
                f,
                "Blocked first_in_word mutex in Screen, this should be impossible"
            ),
            Self::OutputBlockedWordInLineMutex => write!(
                f,
                "Blocked word_in_line mutex in Screen, this should be impossible"
            ),
            Self::OutputMoveCursorIllegalPosition => write!(f, "{}", VM_ERRORS[17]),
        }
    }
}
