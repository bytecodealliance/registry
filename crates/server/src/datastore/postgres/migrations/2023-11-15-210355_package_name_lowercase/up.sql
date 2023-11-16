-- Your SQL goes here
CREATE UNIQUE INDEX logs_package_name_lowercase ON logs (LOWER(name));
