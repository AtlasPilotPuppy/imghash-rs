use image::ImageError;

use crate::{
    convert::Convert,
    math::{dct2_over_matrix, median, Axis},
    ImageHash, ImageHasher,
};
use std::path::Path;

pub struct PerceptualHasher {
    pub width: u32,
    pub height: u32,
    pub factor: u32,
}

impl ImageHasher for PerceptualHasher {
    fn hash_from_path(&self, path: &Path) -> Result<ImageHash, ImageError> {
        match image::io::Reader::open(path)?.decode() {
            Ok(img) => Ok(self.hash_from_img(&img)),
            Err(e) => Err(e),
        }
    }

    fn hash_from_img(&self, img: &image::DynamicImage) -> ImageHash {
        let high_freq = self.convert(img, self.width * self.factor, self.height * self.factor);

        // convert the higher frequency image to a matrix
        let high_freq_bytes = high_freq.as_bytes().to_vec();
        let high_freq_matrix: Vec<Vec<f64>> = high_freq_bytes
            .chunks((self.width * self.factor) as usize)
            .map(|x| x.iter().map(|x| *x as f64).collect::<Vec<f64>>())
            .collect();

        // now we compute the DCT for each column and then for each row
        let dct_matrix = dct2_over_matrix(
            &dct2_over_matrix(&high_freq_matrix, Axis::Column),
            Axis::Row,
        );

        // now we rescale the dct matrix to the actual given width and height
        let scaled_matrix: Vec<Vec<f64>> = dct_matrix
            .iter()
            .take(self.height as usize)
            .map(|row| row.iter().take(self.width as usize).cloned().collect())
            .collect();

        // compute the median over the flattend matrix
        let flattened: Vec<f64> = scaled_matrix.iter().flatten().copied().collect();
        let median = median(&flattened).unwrap();

        // compare each pixel of our scaled image to the mean
        let mut bits = vec![vec![false; self.width as usize]; self.height as usize];
        for (i, row) in scaled_matrix.iter().enumerate() {
            for (j, pixel) in row.iter().enumerate() {
                bits[i][j] = *pixel > median;
            }
        }

        ImageHash { matrix: bits }
    }
}

impl Default for PerceptualHasher {
    fn default() -> Self {
        PerceptualHasher {
            width: 8,
            height: 8,
            factor: 4,
        }
    }
}

impl Convert for PerceptualHasher {}

#[cfg(test)]
mod tests {
    use image::io::Reader as ImageReader;

    use super::*;

    const TEST_IMG: &str = "./data/img/test.png";

    #[test]
    fn test_perceptual_hash_from_img() {
        // Arrange
        let img = ImageReader::open(Path::new(TEST_IMG))
            .unwrap()
            .decode()
            .unwrap();

        let hasher = PerceptualHasher {
            ..Default::default()
        };

        // Act
        let hash = hasher.hash_from_img(&img);

        // Assert
        assert_eq!(hash.python_safe_encode(), "157d1d1b193c7c1c")
    }

    #[test]
    fn test_perceptual_hash_from_path() {
        // Arrange
        let hasher = PerceptualHasher {
            ..Default::default()
        };

        // Act
        let hash = hasher.hash_from_path(Path::new(TEST_IMG));

        // Assert
        match hash {
            Ok(hash) => assert_eq!(hash.python_safe_encode(), "157d1d1b193c7c1c"),
            Err(err) => panic!("could not read image: {:?}", err),
        }
    }

    #[test]
    fn test_perceptual_hash_from_nonexisting_path() {
        // Arrange
        let hasher = PerceptualHasher {
            ..Default::default()
        };

        // Act
        let hash = hasher.hash_from_path(Path::new("./does/not/exist.png"));

        // Assert
        match hash {
            Ok(hash) => panic!("found hash for non-existing image: {:?}", hash),
            Err(_) => (),
        }
    }
}
