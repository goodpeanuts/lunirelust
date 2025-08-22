/// Image data structure for storing images
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageData {
    /// Name of the image
    pub name: String,
    /// MIME type of the image
    pub mime: String,
    /// Raw image bytes
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct UploadImageDto {
    /// The name of this image
    pub id: String,

    /// The image files to upload
    pub files: Vec<ImageData>,
}
