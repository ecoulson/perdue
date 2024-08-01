use std::sync::Arc;

use mock_http::TestServer;
use perdue::{college::{GraduateStudent, Office}, liberal_arts::LiberalArtsScrapper, scraper::scrape_college};
use pretty_assertions::assert_eq;
use reqwest::Client;
use tiny_http::Response;

async fn invoke_scrape_college(scraper: Arc<LiberalArtsScrapper>) -> Vec<Vec<GraduateStudent>> {
    scrape_college(scraper)
        .await
        .expect("Should parse students")
        .into_iter()
        .map(|x| x.into_iter().map(|y| y.unwrap()).collect())
        .collect()
}

#[tokio::test]
async fn should_fetch_liberal_arts_student() {
    let server = TestServer::new();
    server.add_response(Response::from_string(r#"
        <!DOCTYPE html>
        <html>
            <body>
                <table>
                    <tbody>
                        <tr class="hidden profile-row">
                            <td><a href="profiles/adam-kotanko.html">Adam Kotanko</a></td>
                            <td>Graduate Student                             // Sociology
                            </td>
                            <td>&nbsp;</td>
                            <td>akotanko@purdue.edu</td>
                            <td>&nbsp;</td>
                        </tr>
                    </tbody>
                </table>
            </body>
        </html>"#));
    let scraper = LiberalArtsScrapper::new(&server.url(), Arc::new(Client::new()));
    let expected_students = vec![vec![GraduateStudent {
        names: vec![String::from("Adam"), String::from("Kotanko")],
        id: String::from("akotanko"),
        email: String::from("akotanko@purdue.edu"),
        office: Office::default(),
        department: String::from("Sociology"),
    }]];

    let students = invoke_scrape_college(scraper).await;


    assert_eq!(students, expected_students);
}
