use std::fmt::Debug;

use phonenumber::country;


#[derive(Debug, Clone)]
pub struct PhoneNumberDomain(pub String);

impl PhoneNumberDomain{
    pub fn parse(number: String) -> Result<PhoneNumberDomain, String>{
        if phonenumber::parse(Some(country::IN), number.clone()).is_ok(){
            Ok(Self(number))
        } else {
            Err(format!("{} is not a valid user email", number))
        }
    }

    pub fn inner(&self) -> String {
        self.0.clone()
    }
}

impl std::fmt::Display for PhoneNumberDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
