DROP INDEX uq_images_filename ON images;

CREATE INDEX idx_images_filename_lookup ON images(filename);
