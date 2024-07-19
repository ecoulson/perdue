use std::sync::Arc;

use axum::{routing::get, Router};
use perdue::{
    agriculture::AgricultureScrapper,
    college::{list_students, store_students, College, Office},
    health::HealthScrapper,
    html::{NameOrder, ScrapperSelectors},
    liberal_arts::LiberalArtsScrapper,
    salary::{process_salaries, store_salaries},
    scrapper::{scrape_college, SinglePageStudentScrapper},
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::{net::TcpListener, task::JoinSet};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// INDIANA DATA SOURCE FROM: https://gateway.ifionline.org/report_builder/Default3a.aspx?rptType=employComp&rpt=EmployComp&rptName=Employee%20Compensation&rpt_unit_in=3186&referrer=byunit#P4072bd793c4545f0aa97626e908ace39_5_oHit0

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "perdue=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let pool_manager = SqliteConnectionManager::file("database/directory");
    let connection_pool = r2d2::Pool::builder()
        .max_size(8)
        .build(pool_manager)
        .unwrap();
    info!("Pipeline Start");
    pipeline(&connection_pool).await;
    info!("Pipeline Done");

    info!("Server is listening.");
    let router = Router::new()
        .route("/", get(list_students))
        .with_state(connection_pool)
        .nest_service(
            "/assets",
            ServeDir::new(format!(
                "{}/assets",
                std::env::current_dir().unwrap().to_str().unwrap()
            )),
        );
    let listener = TcpListener::bind("0.0.0.0:7777").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

async fn pipeline(connection_pool: &Pool<SqliteConnectionManager>) {
    let client = Arc::new(reqwest::Client::new());
    connection_pool
        .get()
        .unwrap()
        .execute(
            "CREATE TABLE IF NOT EXISTS Students (
            Id VARCHAR PRIMARY KEY,
            Name VARCHAR,
            Email VARCHAR,
            Department VARCHAR,
            Office VARCHAR
            )",
            [],
        )
        .unwrap();
    connection_pool
        .get()
        .unwrap()
        .execute(
            "CREATE INDEX IF NOT EXISTS StudentsByName ON Students (
                Name
            )",
            [],
        )
        .unwrap();
    connection_pool
        .get()
        .unwrap()
        .execute(
            "CREATE TABLE IF NOT EXISTS Salaries (
            StudentId VARCHAR,
            Year INTEGER,
            AmountUsd INTEGER,
            PRIMARY KEY (StudentId, Year)
            )",
            [],
        )
        .unwrap();

    info!("Processing students...");
    let mut scrape_tasks = JoinSet::new();

    info!("Scraping college of agriculture...");
    let agriculture_college = College {
        base_url: String::from(
            "https://ag.purdue.edu/api/pi/2021/api/Directory/ListStaffDirectory",
        ),
        default_department: String::from("School of Agriculture"),
        default_office: Office::default(),
    };
    scrape_tasks.spawn(scrape_college(Arc::new(AgricultureScrapper {
        http_client: client.clone(),
        base_url: agriculture_college.base_url,
    })));

    info!("Scraping college of education...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://education.purdue.edu/graduate-directory/"),
            default_department: String::from("School of Education"),
            default_office: Office::default(),
        },
        delimiter: String::from(" "),
        order: NameOrder::FirstLast,
        allowed_positions: vec![String::from("Graduate Student")],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".grad-directory-archive-container"),
            position_selector: Some(String::from(".position")),
            name_selector: Some(vec![String::from(".grad-directory-archive-info h2")]),
            email_selector: Some(String::from(".grad-directory-archive-contact a")),
            department_selector: Some(String::from(".department")),
            location_selector: None,
        },
    })));

    info!("Scraping college of health...");
    scrape_tasks.spawn(scrape_college(HealthScrapper::new(
        "https://hhs.purdue.edu/wp-admin/admin-ajax.php",
        client.clone(),
    )));

    info!("Scraping college of liberal arts...");
    let liberal_arts_college = College {
        base_url: String::from("https://cla.purdue.edu/directory/"),
        default_office: Office::default(),
        default_department: String::from("School of Liberal Arts"),
    };
    scrape_tasks.spawn(scrape_college(Arc::new(LiberalArtsScrapper {
        client: client.clone(),
        url: liberal_arts_college.base_url,
    })));

    info!("Scraping college of pharmacy...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from(
                "https://www.pharmacy.purdue.edu/directory?name=&dept=&type=gradstudent",
            ),
            default_department: String::from("School of Pharmacy"),
            default_office: Office::default(),
        },
        delimiter: String::from(" "),
        allowed_positions: vec![],
        order: NameOrder::FirstLast,
        selector: ScrapperSelectors {
            directory_row_selector: String::from("table tbody tr"),
            name_selector: Some(vec![String::from("td:nth-child(1)")]),
            position_selector: Some(String::from("td:nth-child(2)")),
            location_selector: Some(String::from("td:nth-child(3)")),
            email_selector: Some(String::from("td:nth-child(5)")),
            department_selector: None,
        },
    })));

    info!("Scraping college of biomedical engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://engineering.purdue.edu/BME/People/GradStudents"),
            default_office: Office {
                building: String::from("Hall of Biomedical Engineering"),
                room: String::from(""),
            },
            default_department: String::from("School of Biomedical Engineering"),
        },
        delimiter: String::from(" "),
        allowed_positions: vec![],
        order: NameOrder::FirstLast,
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selector: Some(vec![
                String::from(".list-name a"),
                String::from(".list-name strong"),
            ]),
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: Some(String::from(".people-list-title")),
        },
    })));

    info!("Scraping college of chemical engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://engineering.purdue.edu/ChE/people/ptGradStudents"),
            default_office: Office {
                building: String::from("Forney Hall of Chemical Engineering"),
                room: String::from(""),
            },
            default_department: String::from("School of Chemical Engineering"),
        },
        delimiter: String::from(" "),
        order: NameOrder::FirstLast,
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selector: Some(vec![String::from(".list-name")]),
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: Some(String::from(".people-list-title")),
        },
    })));

    info!("Scraping college of engineering education...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://engineering.purdue.edu/ENE/People/GraduateStudents"),
            default_office: Office {
                building: String::from("Armstrong Hall"),
                room: String::from(""),
            },
            default_department: String::from("School of Engineering Education"),
        },
        order: NameOrder::FirstLast,
        delimiter: String::from(" "),
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selector: Some(vec![
                String::from(".list-name a"),
                String::from(".list-name strong"),
            ]),
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: Some(String::from(".title")),
        },
    })));

    info!("Scraping college of environmental and ecological engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://engineering.purdue.edu/EEE/People/Graduate"),
            default_office: Office::default(),
            default_department: String::from("School of Environmental and Ecological Engineering"),
        },
        order: NameOrder::FirstLast,
        delimiter: String::from(" "),
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selector: Some(vec![
                String::from(".list-name a"),
                String::from(".list-name strong"),
            ]),
            department_selector: None,
            email_selector: Some(String::from(".people-list-pyEmail a")),
            location_selector: None,
            position_selector: Some(String::from(".people-list-title")),
        },
    })));

    info!("Scraping college of industrial engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://engineering.purdue.edu/IE/people/Grad"),
            default_office: Office {
                building: String::from("Grissom Hall"),
                room: String::from(""),
            },
            default_department: String::from("School of Industrial Engineering"),
        },
        delimiter: String::from(" "),
        order: NameOrder::FirstLast,
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selector: Some(vec![
                String::from(".list-name a"),
                String::from(".list-name span"),
            ]),
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: Some(String::from(".people-list-title")),
        },
    })));

    info!("Scraping college of materials engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from(
                "https://engineering.purdue.edu/MSE/academics/graduate/graduate-directory/index_html",
            ),
            default_office: Office {
                building: String::from(""),
                room: String::from(""),
            },
            default_department: String::from("School of Materials Engineering"),
        },
        delimiter: String::from(" "),
        order: NameOrder::FirstLast,
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".mse-grad-card"),
            name_selector: Some(vec![String::from("h1")]),
            department_selector: None,
            email_selector: Some(String::from("a")),
            location_selector: None,
            position_selector: None,
        },
    })));

    info!("Scraping college of nuclear engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://engineering.purdue.edu/NE/people/grads"),
            default_office: Office {
                building: String::from(""),
                room: String::from(""),
            },
            default_department: String::from("School of Nuclear Engineering"),
        },
        delimiter: String::from(" "),
        order: NameOrder::FirstLast,
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selector: Some(vec![
                String::from(".list-name a"),
                String::from(".list-name strong"),
            ]),
            department_selector: None,
            email_selector: Some(String::from(".email")),
            location_selector: None,
            position_selector: None,
        },
    })));

    info!("Scraping college of biological sciences...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://www.bio.purdue.edu/People/graduate_students.html"),
            default_office: Office {
                building: String::from("LILY"),
                room: String::from(""),
            },
            default_department: String::from("School of Biological Sciences"),
        },
        order: NameOrder::FirstLast,
        delimiter: String::from(" "),
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from("#container .element"),
            name_selector: Some(vec![String::from("h2")]),
            department_selector: None,
            email_selector: Some(String::from("div:nth-child(2) p:nth-child(6) a")),
            location_selector: Some(String::from("div:nth-child(2) p:nth-child(4)")),
            position_selector: None,
        },
    })));

    info!("Scraping college of chemical sciences...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://www.chem.purdue.edu/people/internal.html"),
            default_office: Office {
                building: String::from("BRWN"),
                room: String::from(""),
            },
            default_department: String::from("Department Of Chemistry"),
        },
        delimiter: String::from(", "),
        order: NameOrder::LastFirst,
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".table tbody tr"),
            name_selector: Some(vec![String::from("td:nth-child(3)")]),
            department_selector: None,
            email_selector: Some(String::from("td:nth-child(4)")),
            location_selector: Some(String::from("td:nth-child(7)")),
            position_selector: None,
        },
    })));

    info!("Scraping college of computer sciences...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://www.cs.purdue.edu/people/graduate-students/index.html"),
            default_office: Office {
                building: String::from("LWSN"),
                room: String::from(""),
            },
            default_department: String::from("Department of Computer Science"),
        },
        order: NameOrder::FirstLast,
        delimiter: String::from(" "),
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".table tbody tr"),
            name_selector: Some(vec![String::from("td:nth-child(1)")]),
            department_selector: None,
            email_selector: Some(String::from("td:nth-child(3) a")),
            location_selector: Some(String::from("td:nth-child(2)")),
            position_selector: None,
        },
    })));

    info!("Scraping college of Earth, Atmospheric, and Planatary Sciences...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://www.eaps.purdue.edu/people/grad/index.php"),
            default_office: Office {
                building: String::from("HAMP"),
                room: String::from(""),
            },
            default_department: String::from("School of EAPS"),
        },
        delimiter: String::from(" "),
        order: NameOrder::FirstLast,
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".PhD .peopleDirectoryPerson"),
            name_selector: Some(vec![String::from(".peopleDirectoryInfo strong")]),
            department_selector: None,
            email_selector: Some(String::from(".peopleDirectoryInfo a")),
            location_selector: Some(String::from(".peopleDirectoryInfo div")),
            position_selector: None,
        },
    })));

    info!("Scraping college of mathematics...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from("https://www.math.purdue.edu/people/gradstudents.html"),
            default_office: Office {
                building: String::from("MATH"),
                room: String::from(""),
            },
            default_department: String::from("Department of Mathematics"),
        },
        delimiter: String::from(" "),
        order: NameOrder::FirstLast,
        allowed_positions: vec![],
        selector: ScrapperSelectors {
            directory_row_selector: String::from("#container .directory-row"),
            name_selector: Some(vec![String::from(".peopleDirectoryName a")]),
            department_selector: None,
            email_selector: Some(String::from(".st_details li a")),
            location_selector: Some(String::from(".st_details li:nth-child(2)")),
            position_selector: None,
        },
    })));

    info!("Scraping college of physics and astronomy...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from(
                "https://www.physics.purdue.edu/php-scripts/people/people_list.php",
            ),
            default_office: Office {
                building: String::from("PHYS"),
                room: String::from(""),
            },
            default_department: String::from("Department of Physics and Astronomy"),
        },
        order: NameOrder::LastFirst,
        allowed_positions: vec![String::from("Graduate Students")],
        delimiter: String::from(", "),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".person-item"),
            name_selector: Some(vec![String::from("h2")]),
            department_selector: None,
            email_selector: Some(String::from(".email_link")),
            location_selector: Some(String::from(".info-box div:nth-child(2) .info")),
            position_selector: Some(String::from("a[data-category=\"graduate\"]")),
        },
    })));

    info!("Scraping college of statistics...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from(
                "https://www.stat.purdue.edu/people/graduate_students/",
            ),
            default_office: Office {
                building: String::from("MATH"),
                room: String::from(""),
            },
            default_department: String::from("Department of Statistics"),
        },
        order: NameOrder::FirstLast,
        allowed_positions: vec![],
        delimiter: String::from(" "),
        selector: ScrapperSelectors {
            directory_row_selector: String::from("#container .element"),
            name_selector: Some(vec![String::from("div h2")]),
            department_selector: None,
            email_selector: Some(String::from("div div p a")),
            location_selector: Some(String::from("div div p:nth-child(1)")),
            position_selector: None,
        },
    })));

    info!("Scraping college of veterinary medice...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            base_url: String::from(
                "https://vet.purdue.edu/directory/index.php?classification=20",
            ),
            default_office: Office {
                building: String::from(""),
                room: String::from(""),
            },
            default_department: String::from("Department of Veterinary Medicine"),
        },
        order: NameOrder::LastFirst,
        allowed_positions: vec![],
        delimiter: String::from(", "),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".profile-entry"),
            name_selector: Some(vec![String::from("div:nth-child(1) a")]),
            department_selector: None,
            email_selector: Some(String::from("div:nth-child(3) a")),
            location_selector: None,
            position_selector: None,
        },
    })));

    while let Some(Ok(Ok(scraped_students_by_page))) = scrape_tasks.join_next().await {
        info!("Storing students...");
        for page in scraped_students_by_page {
            store_students(&page, &connection_pool);
        }
    }

    info!("Done storing students...");
    info!("Done processing students...");
    info!("Processing salaries...");
    let salaries = process_salaries(connection_pool);
    store_salaries(&salaries, connection_pool);
    info!("Done processing salaries...");
}
