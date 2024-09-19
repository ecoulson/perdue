use std::sync::Arc;

use tokio::task::JoinSet;

use crate::{
    agriculture::AgricultureScraper,
    college::{store_students, College, Office},
    health::HealthScrapper,
    html::ScrapperSelectors,
    liberal_arts::LiberalArtsScrapper,
    parser::{
        BiologicalSciencesParser, ChemicalSciencesParser, DefaultRowParser, PharmacyParser,
        PhysicsAndAstronomyParser, StatisticsParser, VeterinaryMedicineParser,
    },
    salary::{process_salaries, store_salaries},
    scraper::{scrape_college, SinglePageStudentScrapper},
    server::ServerState,
};

pub fn start_pipeline(state: Arc<ServerState>) {
    tokio::spawn(async move {
        println!("Pipeline Start");
        run(&state).await;
        println!("Pipeline Done");
    });
}

async fn run(state: &Arc<ServerState>) {
    let client = Arc::new(reqwest::Client::new());

    println!("Processing students...");
    let mut scrape_tasks = JoinSet::new();

    println!("Scraping college of agriculture...");
    let agriculture_college = College {
        id: String::from("0"),
        name: String::from("College of Agriculture"),
        base_url: String::from(
            "https://ag.purdue.edu/api/pi/2021/api/Directory/ListStaffDirectory",
        ),
        default_department: String::from("School of Agriculture"),
        default_office: Office::default(),
    };
    scrape_tasks.spawn(scrape_college(Arc::new(AgricultureScraper {
        http_client: client.clone(),
        base_url: agriculture_college.base_url,
    })));

    println!("Scraping college of education...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("1"),
            name: String::from("College of Education"),
            base_url: String::from("https://education.purdue.edu/graduate-directory/"),
            default_department: String::from("School of Education"),
            default_office: Office::default(),
        },
        parser: Box::new(DefaultRowParser {
            default_department: String::from("School of Education"),
            default_office: Office::default(),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".grad-directory-archive-container"),
            position_selector: Some(String::from(".position")),
            name_selectors: vec![String::from(".grad-directory-archive-info h2")],
            email_selector: Some(String::from(".grad-directory-archive-contact a")),
            department_selector: Some(String::from(".department")),
            location_selector: None,
        },
    })));

    println!("Scraping college of health...");
    scrape_tasks.spawn(scrape_college(HealthScrapper::new(
        "https://hhs.purdue.edu/wp-admin/admin-ajax.php",
        client.clone(),
    )));

    println!("Scraping college of liberal arts...");
    let liberal_arts_college = College {
        id: String::from("2"),
        name: String::from("College of Liberal Arts"),
        base_url: String::from("https://cla.purdue.edu/directory/"),
        default_office: Office::default(),
        default_department: String::from("School of Liberal Arts"),
    };
    scrape_tasks.spawn(scrape_college(Arc::new(LiberalArtsScrapper {
        client: client.clone(),
        url: liberal_arts_college.base_url,
    })));

    println!("Scraping college of pharmacy...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("3"),
            name: String::from("College of Pharmacy"),
            base_url: String::from(
                "https://www.pharmacy.purdue.edu/directory?name=&dept=&type=gradstudent",
            ),
            default_department: String::from("School of Pharmacy"),
            default_office: Office::default(),
        },
        parser: Box::new(PharmacyParser {}),
        selector: ScrapperSelectors {
            directory_row_selector: String::from("table tbody tr"),
            name_selectors: vec![String::from("td:nth-child(1)")],
            position_selector: Some(String::from("td:nth-child(2)")),
            location_selector: Some(String::from("td:nth-child(3)")),
            email_selector: Some(String::from("td:nth-child(5) a")),
            department_selector: None,
        },
    })));

    println!("Scraping college of biomedical engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("4"),
            name: String::from("College of Biomedial Engineering"),
            base_url: String::from("https://engineering.purdue.edu/BME/People/GradStudents"),
            default_office: Office {
                building: String::from("Hall of Biomedical Engineering"),
                room: String::from(""),
            },
            default_department: String::from("School of Biomedical Engineering"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from("Hall of Biomedical Engineering"),
                room: String::from(""),
            },
            default_department: String::from("School of Biomedical Engineering"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selectors: vec![
                String::from(".list-name a"),
                String::from(".list-name strong"),
            ],
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: Some(String::from(".people-list-title")),
        },
    })));

    println!("Scraping college of chemical engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("5"),
            name: String::from("College of Chemical Engineering"),
            base_url: String::from("https://engineering.purdue.edu/ChE/people/ptGradStudents"),
            default_office: Office {
                building: String::from("Forney Hall of Chemical Engineering"),
                room: String::from(""),
            },
            default_department: String::from("School of Chemical Engineering"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from("Forney Hall of Chemical Engineering"),
                room: String::from(""),
            },
            default_department: String::from("School of Chemical Engineering"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selectors: vec![String::from(".list-name")],
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: Some(String::from(".people-list-title")),
        },
    })));

    println!("Scraping college of engineering education...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("6"),
            name: String::from("College of Engineering Education"),
            base_url: String::from("https://engineering.purdue.edu/ENE/People/GraduateStudents"),
            default_office: Office {
                building: String::from("Armstrong Hall"),
                room: String::from(""),
            },
            default_department: String::from("School of Engineering Education"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from("Armstrong Hall"),
                room: String::from(""),
            },
            default_department: String::from("School of Engineering Education"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selectors: vec![
                String::from(".list-name a"),
                String::from(".list-name strong"),
            ],
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: Some(String::from(".title")),
        },
    })));

    println!("Scraping college of environmental and ecological engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("7"),
            name: String::from("College of Ecological Engineering"),
            base_url: String::from("https://engineering.purdue.edu/EEE/People/Graduate"),
            default_office: Office::default(),
            default_department: String::from("School of Environmental and Ecological Engineering"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office::default(),
            default_department: String::from("School of Environmental and Ecological Engineering"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selectors: vec![
                String::from(".list-name a"),
                String::from(".list-name strong"),
            ],
            department_selector: None,
            email_selector: Some(String::from(".people-list-pyEmail a")),
            location_selector: None,
            position_selector: Some(String::from(".people-list-title")),
        },
    })));

    println!("Scraping college of industrial engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("8"),
            name: String::from("College of Industrial Engineering"),
            base_url: String::from("https://engineering.purdue.edu/IE/people/Grad"),
            default_office: Office {
                building: String::from("Grissom Hall"),
                room: String::from(""),
            },
            default_department: String::from("School of Industrial Engineering"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from("Grissom Hall"),
                room: String::from(""),
            },
            default_department: String::from("School of Industrial Engineering"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selectors: vec![
                String::from(".list-name a"),
                String::from(".list-name span"),
            ],
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: Some(String::from(".people-list-title")),
        },
    })));

    println!("Scraping college of materials engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("9"),
            name: String::from("College of Materials Engineering"),
            base_url: String::from(
                "https://engineering.purdue.edu/MSE/academics/graduate/graduate-directory/index_html",
            ),
            default_office: Office {
                building: String::from(""),
                room: String::from(""),
            },
            default_department: String::from("School of Materials Engineering"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from(""),
                room: String::from(""),
            },
            default_department: String::from("School of Materials Engineering"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".mse-grad-card"),
            name_selectors: vec![String::from("h1")],
            department_selector: None,
            email_selector: Some(String::from("a")),
            location_selector: None,
            position_selector: None,
        },
    })));

    println!("Scraping college of nuclear engineering...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("10"),
            name: String::from("College of Nuclear Engineering"),
            base_url: String::from("https://engineering.purdue.edu/NE/people/grads"),
            default_office: Office {
                building: String::from(""),
                room: String::from(""),
            },
            default_department: String::from("School of Nuclear Engineering"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from(""),
                room: String::from(""),
            },
            default_department: String::from("School of Nuclear Engineering"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".people-list .row"),
            name_selectors: vec![
                String::from(".list-name a"),
                String::from(".list-name strong"),
            ],
            department_selector: None,
            email_selector: Some(String::from(".email a")),
            location_selector: None,
            position_selector: None,
        },
    })));

    println!("Scraping college of biological sciences...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("11"),
            name: String::from("College of Biological Sciences"),
            base_url: String::from("https://www.bio.purdue.edu/People/graduate_students.html"),
            default_office: Office {
                building: String::from("LILY"),
                room: String::from(""),
            },
            default_department: String::from("School of Biological Sciences"),
        },
        parser: Box::new(BiologicalSciencesParser {}),
        selector: ScrapperSelectors {
            directory_row_selector: String::from("#container .element"),
            name_selectors: vec![String::from("h2")],
            department_selector: None,
            email_selector: Some(String::from("div:nth-child(2) p:nth-child(6) a")),
            location_selector: Some(String::from("div:nth-child(2) p:nth-child(4)")),
            position_selector: None,
        },
    })));

    println!("Scraping college of chemical sciences...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("12"),
            name: String::from("College of Chemical Sciences"),
            base_url: String::from("https://www.chem.purdue.edu/people/internal.html"),
            default_office: Office {
                building: String::from("BRWN"),
                room: String::from(""),
            },
            default_department: String::from("Department Of Chemistry"),
        },
        parser: Box::new(ChemicalSciencesParser {}),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".table tbody tr"),
            name_selectors: vec![String::from("td:nth-child(3)")],
            department_selector: None,
            email_selector: Some(String::from("td:nth-child(4) a")),
            location_selector: Some(String::from("td:nth-child(7)")),
            position_selector: None,
        },
    })));

    println!("Scraping college of computer sciences...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("12"),
            name: String::from("College of Computer Sciences"),
            base_url: String::from("https://www.cs.purdue.edu/people/graduate-students/index.html"),
            default_office: Office {
                building: String::from("LWSN"),
                room: String::from(""),
            },
            default_department: String::from("Department of Computer Science"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from("LWSN"),
                room: String::from(""),
            },
            default_department: String::from("Department of Computer Science"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".table tbody tr"),
            name_selectors: vec![String::from("td:nth-child(1)")],
            department_selector: None,
            email_selector: Some(String::from("td:nth-child(3) a")),
            location_selector: Some(String::from("td:nth-child(2)")),
            position_selector: None,
        },
    })));

    println!("Scraping college of Earth, Atmospheric, and Planatary Sciences...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("13"),
            name: String::from("College of Earth, Atmospheric, and Planatary Sciences"),
            base_url: String::from("https://www.eaps.purdue.edu/people/grad/index.php"),
            default_office: Office {
                building: String::from("HAMP"),
                room: String::from(""),
            },
            default_department: String::from("School of EAPS"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from("HAMP"),
                room: String::from(""),
            },
            default_department: String::from("School of EAPS"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".PhD .peopleDirectoryPerson"),
            name_selectors: vec![String::from(".peopleDirectoryInfo strong")],
            department_selector: None,
            email_selector: Some(String::from(".peopleDirectoryInfo a")),
            location_selector: Some(String::from(".peopleDirectoryInfo div")),
            position_selector: None,
        },
    })));

    println!("Scraping college of mathematics...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("14"),
            name: String::from("College of Mathematics"),
            base_url: String::from("https://www.math.purdue.edu/people/gradstudents.html"),
            default_office: Office {
                building: String::from("MATH"),
                room: String::from(""),
            },
            default_department: String::from("Department of Mathematics"),
        },
        parser: Box::new(DefaultRowParser {
            default_office: Office {
                building: String::from("MATH"),
                room: String::from(""),
            },
            default_department: String::from("Department of Mathematics"),
        }),
        selector: ScrapperSelectors {
            directory_row_selector: String::from("#container .directory-row"),
            name_selectors: vec![String::from(".peopleDirectoryName a")],
            department_selector: None,
            email_selector: Some(String::from(".st_details li a")),
            location_selector: Some(String::from(".st_details li:nth-child(2)")),
            position_selector: None,
        },
    })));

    println!("Scraping college of physics and astronomy...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("15"),
            name: String::from("College of Physics and Astronomy"),
            base_url: String::from(
                "https://www.physics.purdue.edu/php-scripts/people/people_list.php",
            ),
            default_office: Office {
                building: String::from("PHYS"),
                room: String::from(""),
            },
            default_department: String::from("Department of Physics and Astronomy"),
        },
        parser: Box::new(PhysicsAndAstronomyParser {}),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".person-item"),
            name_selectors: vec![String::from("h2")],
            department_selector: None,
            email_selector: Some(String::from(".email_link")),
            location_selector: Some(String::from(".info-box div:nth-child(2) .info")),
            position_selector: Some(String::from("a[data-category=\"graduate\"]")),
        },
    })));

    println!("Scraping college of statistics...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("16"),
            name: String::from("College of Statistics"),
            base_url: String::from("https://www.stat.purdue.edu/people/graduate_students/"),
            default_office: Office {
                building: String::from("MATH"),
                room: String::from(""),
            },
            default_department: String::from("Department of Statistics"),
        },
        parser: Box::new(StatisticsParser {}),
        selector: ScrapperSelectors {
            directory_row_selector: String::from("#container .element"),
            name_selectors: vec![String::from("div h2")],
            department_selector: None,
            email_selector: Some(String::from("div div p a")),
            location_selector: Some(String::from("div div p:nth-child(1)")),
            position_selector: None,
        },
    })));

    println!("Scraping college of veterinary medice...");
    scrape_tasks.spawn(scrape_college(Arc::new(SinglePageStudentScrapper {
        client: client.clone(),
        college: College {
            id: String::from("17"),
            name: String::from("College of Vererinary Medicine"),
            base_url: String::from("https://vet.purdue.edu/directory/index.php?classification=20"),
            default_office: Office {
                building: String::from(""),
                room: String::from(""),
            },
            default_department: String::from("Department of Veterinary Medicine"),
        },
        parser: Box::new(VeterinaryMedicineParser {}),
        selector: ScrapperSelectors {
            directory_row_selector: String::from(".profile-entry"),
            name_selectors: vec![String::from("div:nth-child(1) a")],
            department_selector: None,
            email_selector: Some(String::from("div:nth-child(3) a")),
            location_selector: None,
            position_selector: None,
        },
    })));

    while let Some(Ok(Ok(scraped_students_by_page))) = scrape_tasks.join_next().await {
        println!("Storing students...");
        for page in scraped_students_by_page {
            store_students(&page, &state.connection_pool);
        }
    }

    println!("Done storing students...");
    println!("Done processing students...");
    println!("Processing salaries...");
    let salaries = process_salaries(&state.connection_pool, &state.configuration.files.salaries_directory);
    store_salaries(&salaries, &state.connection_pool);
    println!("Done processing salaries...");
}
