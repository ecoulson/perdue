CREATE TABLE IF NOT EXISTS College (
    Id VARCHAR PRIMARY KEY,
    Name VARCHAR
);

ALTER TABLE Students ADD COLUMN CollegeId VARCHAR REFERENCES College(Id);
