CREATE TABLE IF NOT EXISTS Offices(
    OfficeId VARCHAR PRIMARY KEY,
    StudentId VARCHAR,
    Building VARCHAR,
    Room VARCHAR,
    FOREIGN KEY(StudentId) REFERENCES Students(Id)
);

CREATE INDEX IF NOT EXISTS OfficesByStudentId ON Offices (
    StudentId
);

CREATE TABLE IF NOT EXISTS Responses(
    ResponseId VARCHAR PRIMARY KEY,
    Timestamp INTEGER,
    StudentId VARCHAR,
    GraduateStudent BOOLEAN,
    FirstAndLastName VARCHAR,
    AcademicUnit VARCHAR,
    PersonalEmail VARCHAR,
    PhoneNumber VARCHAR,
    MailingList VARCHAR,
    KeyIssues TEXT,
    FOREIGN KEY(StudentId) REFERENCES Students(Id)
);

ALTER TABLE Students DROP COLUMN Office;
