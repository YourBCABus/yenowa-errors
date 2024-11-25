-- Setup Error Table
CREATE TABLE default.errs
(
    err_id UUID NOT NULL DEFAULT generateUUIDv4(),
    service TEXT NOT NULL,
    subservice TEXT NOT NULL,

    error_message TEXT NOT NULL,
    error_data_json TEXT NOT NULL,
    timestamp DateTime NOT NULL DEFAULT now(),
) PRIMARY KEY err_id;

-- Setup user
CREATE USER err_reporter IDENTIFIED WITH bcrypt_password BY '<password>';
GRANT ALL ON errs TO 'err_reporter';
