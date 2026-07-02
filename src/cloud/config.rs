pub const FIREBASE_API_KEY: &str = match option_env!("VENTUS_FIREBASE_API_KEY") {
    Some(v) => v,
    None => "",
};

pub const FIREBASE_PROJECT_ID: &str = match option_env!("VENTUS_FIREBASE_PROJECT_ID") {
    Some(v) => v,
    None => "",
};

pub const FIREBASE_AUTH_DOMAIN: &str = match option_env!("VENTUS_FIREBASE_AUTH_DOMAIN") {
    Some(v) => v,
    None => "",
};

pub const FIREBASE_APP_ID: &str = match option_env!("VENTUS_FIREBASE_APP_ID") {
    Some(v) => v,
    None => "",
};

pub const CLOUDINARY_CLOUD_NAME: &str = match option_env!("VENTUS_CLOUDINARY_CLOUD_NAME") {
    Some(v) => v,
    None => "",
};

pub const CLOUDINARY_UPLOAD_PRESET: &str = match option_env!("VENTUS_CLOUDINARY_UPLOAD_PRESET") {
    Some(v) => v,
    None => "",
};

pub fn is_configured() -> bool {
    !FIREBASE_API_KEY.is_empty() && !FIREBASE_PROJECT_ID.is_empty()
}

pub fn cloudinary_configured() -> bool {
    !CLOUDINARY_CLOUD_NAME.is_empty() && !CLOUDINARY_UPLOAD_PRESET.is_empty()
}
