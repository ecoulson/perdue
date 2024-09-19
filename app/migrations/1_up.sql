CREATE TABLE IF NOT EXISTS Students (
            Id VARCHAR PRIMARY KEY,
            Name VARCHAR,
            Email VARCHAR,
            Department VARCHAR,
            Office VARCHAR
            );

CREATE INDEX IF NOT EXISTS StudentsByName ON Students (
                Name
            );

CREATE TABLE IF NOT EXISTS Salaries (
            StudentId VARCHAR,
            Year INTEGER,
            AmountUsd INTEGER,
            PRIMARY KEY (StudentId, Year)
            );
