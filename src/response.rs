/// Gemini possible responses.
///
/// Parameters are formated so first is the <META> field
/// and the second is the <BODY> field.
#[derive(Clone, Debug)]
pub enum Response {
    Input(String),
    SensitiveInput(String),

    Success(String, Vec<u8>),

    RedirectPermanent(String),
    RedirectTemporary(String),

    TemporaryFailure(String),
    ServerUnavailable(String),
    CgiError(String),
    ProxyError(String),
    SlowDown(String),

    PermanentFailure(String),
    NotFound(String),
    Gone(String),
    ProxyRequestRefused(String),

    BadRequest(String),

    ClientCertificationRequired(String),
    ClientCertificationUnauthorized(String),
    ClientCertificateNotValid(String),
}

impl Response {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut result = Vec::<u8>::new();
        match self {
            Response::Input(meta) => result.extend_from_slice(format!("10 {}\r\n", meta).as_bytes()),
            Response::SensitiveInput(meta) => {
                result.extend_from_slice(format!("11 {}\r\n", meta).as_bytes())
            }

            Response::Success(meta, body) => {
                result.extend_from_slice(format!("20 {}\r\n", meta).as_bytes());
                result.extend(body);
            }

            Response::RedirectPermanent(meta) => {
                result.extend_from_slice(format!("30 {}\r\n", meta).as_bytes())
            }
            Response::RedirectTemporary(meta) => {
                result.extend_from_slice(format!("31 {}\r\n", meta).as_bytes())
            }

            Response::TemporaryFailure(meta) => {
                result.extend_from_slice(format!("40 {}\r\n", meta).as_bytes())
            }
            Response::ServerUnavailable(meta) => {
                result.extend_from_slice(format!("41 {}\r\n", meta).as_bytes())
            }
            Response::CgiError(meta) => result.extend_from_slice(format!("42 {}\r\n", meta).as_bytes()),
            Response::ProxyError(meta) => {
                result.extend_from_slice(format!("43 {}\r\n", meta).as_bytes())
            }
            Response::SlowDown(meta) => result.extend_from_slice(format!("44 {}\r\n", meta).as_bytes()),

            Response::PermanentFailure(meta) => {
                result.extend_from_slice(format!("50 {}\r\n", meta).as_bytes())
            }
            Response::NotFound(meta) => result.extend_from_slice(format!("50 {}\r\n", meta).as_bytes()),
            Response::Gone(meta) => result.extend_from_slice(format!("51 {}\r\n", meta).as_bytes()),
            Response::ProxyRequestRefused(meta) => {
                result.extend_from_slice(format!("52 {}\r\n", meta).as_bytes())
            }

            Response::BadRequest(meta) => {
                result.extend_from_slice(format!("59 {}\r\n", meta).as_bytes())
            }

            Response::ClientCertificationRequired(meta) => {
                result.extend_from_slice(format!("60 {}\r\n", meta).as_bytes())
            }
            Response::ClientCertificationUnauthorized(meta) => {
                result.extend_from_slice(format!("61 {}\r\n", meta).as_bytes())
            }
            Response::ClientCertificateNotValid(meta) => {
                result.extend_from_slice(format!("62 {}\r\n", meta).as_bytes())
            }
        }
        result
    }
}
