DROP INDEX IF EXISTS uq_images_filename;

CREATE INDEX IF NOT EXISTS idx_images_filename_lookup ON images(filename);
