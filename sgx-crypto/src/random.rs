pub use mbedtls::rng::{Random, EntropyCallback};

#[cfg(not(target_env = "sgx"))]
pub fn entropy_new<'a>() -> mbedtls::rng::OsEntropy<'a> {
    mbedtls::rng::OsEntropy::new()
}

#[cfg(target_env = "sgx")]
pub struct Rng {
    pub inner: mbedtls::rng::Rdrand,
}

#[cfg(target_env = "sgx")]
impl Rng {
    pub fn new() -> Self {
        Self {
            inner: mbedtls::rng::Rdrand,
        }
    }
    pub fn random(&mut self, data: &mut [u8]) -> super::Result<()> {
        self.inner.random(data).expect("generate random failed");
        Ok(())
    }
}

#[cfg(not(target_env = "sgx"))]
pub struct Rng<'a> {
    pub inner: mbedtls::rng::CtrDrbg<'a>,
}

#[cfg(not(target_env = "sgx"))]
impl<'a> Rng<'a> {
    pub fn new(source: &'a mut impl EntropyCallback) -> super::Result<Self> {
        Ok(Self {
            inner: mbedtls::rng::CtrDrbg::new(source, None)?,
        })
    }
}
