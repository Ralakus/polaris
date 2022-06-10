macro_rules! status_codes {
    ({ $( $code:ident = $value:expr , )+ }) => {
        /// Macro was used to generate this bit of code.
        ///
        /// All of the valid Gemini status codes are included in this list as its own enum
        /// value. Macro is used due to needing to generate to and from u8 mappings.
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
        pub enum Code {
            $(
                $code = $value ,
            )+
            Invalid = 0,
        }

        impl From<u8> for Code {
            fn from(value: u8) -> Self {
                match value {
                    $(
                        $value => Self::$code,
                    )+
                    _ => Self::Invalid
                }
            }
        }

        impl std::fmt::Display for Code {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $(
                        Self::$code => write!(f, stringify!($code)),
                    )+
                    _ => write!(f, "Invalid")
                }
            }
        }

        impl Code {
            /// If code is between `low` inclusive and `high` inclusive
            #[inline]
            pub fn in_range(&self, low: u8, high: u8) -> bool {
                (*self as u8) >= low && (*self as u8) <= high
            }
            /// If code is input related
            #[inline]
            pub fn is_input(&self) -> bool {
                self.in_range(10, 19)
            }
            /// If code is success related
            #[inline]
            pub fn is_success(&self) -> bool {
                self.in_range(20, 29)
            }
            /// If code is redirect related
            #[inline]
            pub fn is_redirect(&self) -> bool {
                self.in_range(30, 39)
            }
            /// If code is temporary failure related
            #[inline]
            pub fn is_temporary_failure(&self) -> bool {
                self.in_range(40, 49)
            }
            // If code is permanent failure related
            #[inline]
            pub fn is_permanent_failure(&self) -> bool {
                self.in_range(50, 59)
            }
            /// If code is client certification related
            #[inline]
            pub fn is_client_certification_failure(&self) -> bool {
                self.in_range(60, 69)
            }
            /// Returns last digit of two digit code
            #[inline]
            pub fn last_digit(&self) -> u8 {
                (*self as u8) % 10
            }
            /// Returns first digit of two digit code
            #[inline]
            pub fn first_digit(&self) -> u8 {
                (*self as u8) / 10
            }
            /// Returns first and last digits of two digit code
            #[inline]
            pub fn digit_pair(&self) -> (u8, u8) {
                (self.first_digit(), self.last_digit())
            }
        }
    };
}

status_codes! ({
    // Input codes
    Input = 10,
    SensitiveInput = 11,

    Input2 = 12,
    Input3 = 13,
    Input4 = 14,
    Input5 = 15,
    Input6 = 16,
    Input7 = 17,
    Input8 = 18,
    Input9 = 19,

    // Success codes
    Success = 20,

    SuccessCode1 = 21,
    SuccessCode2 = 22,
    SuccessCode3 = 23,
    SuccessCode4 = 24,
    SuccessCode5 = 25,
    SuccessCode6 = 26,
    SuccessCode7 = 27,
    SuccessCode8 = 28,
    SuccessCode9 = 29,

    // Redirect codes
    RedirectPermanent = 30,
    RedirectTemporary = 31,

    RedirectCode2 = 32,
    RedirectCode3 = 33,
    RedirectCode4 = 34,
    RedirectCode5 = 35,
    RedirectCode6 = 36,
    RedirectCode7 = 37,
    RedirectCode8 = 38,
    RedirectCode9 = 39,

    // Temporary failure codes
    TemporaryFailure = 40,
    ServerUnavailable = 41,
    CgiError = 42,
    ProxyError = 43,
    SlowDown = 44,

    TemporaryFailureCode5 = 45,
    TemporaryFailureCode6 = 46,
    TemporaryFailureCode7 = 47,
    TemporaryFailureCode8 = 48,
    TemporaryFailureCode9 = 49,

    // Permanent failure codes
    PermanentFailure = 50,
    NotFound = 51,
    Gone = 52,
    ProxyRequestRefused = 53,

    PermanentFailureCode4 = 54,
    PermanentFailureCode5 = 55,
    PermanentFailureCode6 = 56,
    PermanentFailureCode7 = 57,
    PermanentFailureCode8 = 58,

    BadRequest = 59,

    // Client certification codes
    ClientCertificationRequired = 60,
    ClientCertificationUnauthorized = 61,
    ClientCertificateNotValid = 62,

    ClientCertificateCode3 = 63,
    ClientCertificateCode4 = 64,
    ClientCertificateCode5 = 65,
    ClientCertificateCode6 = 66,
    ClientCertificateCode7 = 67,
    ClientCertificateCode8 = 68,
    ClientCertificateCode9 = 69,
});
