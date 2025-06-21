use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::{ prelude::FromRow, QueryBuilder};
use uuid::Uuid;

use crate::{models::UserData, AppState};
 
// _______________________________________ User related routes _______________________________________
#[derive(Debug, Deserialize)]
pub struct CreateSnippetRequest {
    pub title: String,
    pub language: String,
}

#[post("/snippets")]
pub async fn create_snippet(
    app_data: web::Data<AppState>, 
    data_json: web::Json<CreateSnippetRequest>,
    user_data: web::ReqData<UserData>,
) -> actix_web::Result<impl Responder> {
    let rec = sqlx::query!(
        r#"
        INSERT INTO snippets_extension.snippets (title, language, owner_id)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        data_json.title,
        data_json.language,
        user_data.id,
    )
    .fetch_one(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"id": rec.id})))
}

#[get("/{userId}/snippets")]
pub async fn get_user_snippets (
    app_data: web::Data<AppState>,
    path: web::Path<Uuid>, 
    user_data: web::ReqData<UserData>,
) -> actix_web::Result<impl Responder> {
    let user_id = path.into_inner();
    let req_user_id = user_data.id;

    let snippets: Vec<SnippetData> = sqlx::query_as!(
        SnippetData,
        r#"
        WITH star_counts AS (
          SELECT 
            snippet_id,
            COUNT(*) AS stars
          FROM snippets_extension.snippet_stars
          GROUP BY snippet_id
        ),
        tag_lists AS (
          SELECT
            st.snippet_id,
            array_agg(DISTINCT t.name) AS tags
          FROM snippets_extension.snippet_tags st
          JOIN snippets_extension.tags t
            ON t.id = st.tag_id
          GROUP BY st.snippet_id
        )
        SELECT
          s.id,
          s.title,
          s.description,
          s.code,
          s.language,
          COALESCE(sc.stars, 0) AS "stars!: i64", -- Built Stars Column (Defaults to 0)
          COALESCE(tl.tags, ARRAY[]::TEXT[]) AS "tags!: Vec<String>" -- Built Tags Column (Defaults to Empty Array of Text)
        FROM snippets_extension.snippets s
        LEFT JOIN star_counts sc
          ON sc.snippet_id = s.id
        LEFT JOIN tag_lists tl
          ON tl.snippet_id = s.id
        WHERE s.owner_id = $1
        ORDER BY s.created_at DESC
        "#,
        user_id
    )
    .fetch_all(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;


    Ok(HttpResponse::Ok().json(
        serde_json::json!({
            "snippets": snippets, 
            "owner": user_id == req_user_id
        })
    ))
}

#[get("/{userId}/snippets/{snippetId}")]
pub async fn get_user_snippet(
    app_data: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    user_data: web::ReqData<UserData>,
) -> actix_web::Result<impl Responder> {
    // destructure the two path params
    let (user_id, snippet_id) = path.into_inner();
    let req_user_id = user_data.id;

    // load exactly one snippet by owner + id
    let snippet = sqlx::query_as!(
        SnippetData,
        r#"
        WITH star_counts AS (
            SELECT
                snippet_id,
                COUNT(*) AS stars
            FROM snippets_extension.snippet_stars
            GROUP BY snippet_id
        ),
        tag_lists AS (
            SELECT
                st.snippet_id,
                array_agg(DISTINCT t.name) AS tags
            FROM snippets_extension.snippet_tags st
            JOIN snippets_extension.tags t
                ON t.id = st.tag_id
            GROUP BY st.snippet_id
        )
        SELECT
            s.id,
            s.title,
            s.description,
            s.code,
            s.language,
            COALESCE(sc.stars, 0) AS "stars!: i64", -- Built Stars Column (Defaults to 0)
            COALESCE(tl.tags, ARRAY[]::TEXT[]) AS "tags!: Vec<String>" -- Built Tags Column (Defaults to Empty Array of Text)
        FROM snippets_extension.snippets s
        LEFT JOIN star_counts sc
            ON sc.snippet_id = s.id
        LEFT JOIN tag_lists tl
            ON tl.snippet_id = s.id
        WHERE
            s.owner_id = $1
        AND 
            s.id = $2
        "#,
        user_id,
        snippet_id
    )
    .fetch_optional(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    // if no snippet, return 404
    let snippet = match snippet {
        Some(s) => s,
        None => return Ok(HttpResponse::NotFound().finish()),
    };

    // respond with the snippet and a flag telling whether the
    // requesting user “owns” it
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "snippet": snippet,
        "owner":   user_id == req_user_id
    })))
}


#[derive(Deserialize)]
pub struct UpdateSnippetRequest {
    pub title: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub tags: Vec<String>
}

#[put("/snippets/{snippetId}")]
pub async fn update_snippet(
    app_data: web::Data<AppState>,
    path: web::Path<Uuid>,
    user_data: web::ReqData<UserData>,
    json_data: web::Json<UpdateSnippetRequest>
) -> actix_web::Result<impl Responder> {
    let snippet_id: Uuid = path.into_inner();
    let user_id = user_data.id;

    let result = sqlx::query!(
        r#"
        UPDATE snippets_extension.snippets
        SET
            title       = $1,
            description = $2,
            code        = $3,
            language    = $4,
            updated_at  = NOW()
        WHERE
            id       = $5
            AND owner_id = $6
        "#,
        json_data.title,
        json_data.description,
        json_data.code,
        json_data.language,
        snippet_id,
        user_id,
    )
    .execute(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if result.rows_affected() == 0 {
        return Ok(HttpResponse::NotFound().json(
            serde_json::json!({ "error": "snippet not found or not owned by you" })
        ));
    }
   
    sqlx::query!(
        r#"
        WITH new_tags AS (
            INSERT INTO snippets_extension.tags(name)
            SELECT unnest($1::text[])
            ON CONFLICT (name) DO NOTHING
            RETURNING id
        ),
        all_tags AS (
            SELECT id
              FROM snippets_extension.tags
             WHERE name = ANY($1::text[])
        ),
        deleted AS (
            DELETE FROM snippets_extension.snippet_tags st
             WHERE st.snippet_id = $2
               AND st.tag_id NOT IN (SELECT id FROM all_tags)
        ),
        inserted AS (
            INSERT INTO snippets_extension.snippet_tags(snippet_id, tag_id)
            SELECT $2, id FROM all_tags
            ON CONFLICT DO NOTHING
        )
        SELECT 1 as unused;
        "#,
        &json_data.tags,
        snippet_id,
    )
    .fetch_one(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().finish())
}

#[delete("/snippets/{snippetId}")]
pub async fn delete_snippet(
    app_data: web::Data<AppState>,
    path: web::Path<Uuid>,
    user_data: web::ReqData<UserData>
) -> actix_web::Result<impl Responder> {
    let snippet_id: Uuid = path.into_inner();
    let user_id = user_data.id;

    let rec = sqlx::query!(
        r#"
        DELETE FROM snippets_extension.snippets
        WHERE id = $1
          AND owner_id = $2
        RETURNING id
        "#,
        snippet_id,
        user_id
    )
    .fetch_optional(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if let Some(_) = rec {
        return Ok(
            HttpResponse::Ok().finish()
        )
    } else {
        return Ok(
            HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("No snippet found with id {}", snippet_id)
            }))
        )
    }
}


#[derive(Deserialize, Serialize, FromRow)]
pub struct SnippetData {
    pub id:          Uuid,
    pub title:       String,
    pub description: Option<String>,
    pub code:        Option<String>,
    pub language:    String,
    pub stars:       i64,
    pub tags:        Vec<String>,
}

#[derive(Deserialize)]
pub struct PageParams {
    pub language: Option<String>,
    pub title:    Option<String>,
    pub page:     Option<u32>,
    pub limit:    Option<u32>,
}
#[derive(Serialize)]
pub struct PageResponse {
    pub total_records: i64,
    pub total_pages:   u32,
    pub current_page:  u32,
    pub records:       Vec<SnippetData>,
}

// _______________________________________ Snippets related routes _______________________________________

#[get("")]
pub async fn get_page_snippets(
    app_data: web::Data<AppState>,
    params:   web::Query<PageParams>,
) -> actix_web::Result<impl Responder> {
    let PageParams { language, title, page, limit } = params.into_inner();
    let current_page = page.unwrap_or(1).max(1);
    let per_page     = limit.unwrap_or(12).clamp(1, 100);
    let offset       = (current_page - 1) * per_page;

    let mut count_qb = QueryBuilder::new(
        r#"
        SELECT COUNT(*) AS total 
        FROM snippets_extension.snippets
        "#
    );

    let mut data_qb = QueryBuilder::new(
        r#"
        SELECT 
            s.id,
            s.title,
            s.description,
            s.code,
            s.language,
            -- count how many stars this snippet has
            COUNT(ss.user_id) AS stars,
            -- collect its tags (empty array if none)
            COALESCE(
              array_agg(DISTINCT t.name) 
              FILTER (WHERE t.name IS NOT NULL),
              ARRAY[]::TEXT[]
            ) AS tags
        FROM snippets_extension.snippets s
        LEFT JOIN snippets_extension.snippet_stars AS ss
          ON ss.snippet_id = s.id
        LEFT JOIN snippets_extension.snippet_tags AS st
          ON st.snippet_id = s.id
        LEFT JOIN snippets_extension.tags AS t
          ON t.id = st.tag_id
        "#
    );

    let mut has_where = false;
    match &language {
        Some(lang) if !lang.is_empty() => {
            let clause = if has_where { " AND language = " } else { " WHERE language = " };
            count_qb.push(clause).push_bind(lang);
            data_qb.push(clause).push_bind(lang);
            has_where = true;
        }
        _ => {}
    }
    
    if let Some(t) = &title {
        let pattern = format!("%{}%", t);
        let clause  = if has_where { " AND title ILIKE " } else { " WHERE title ILIKE " };
        count_qb.push(clause).push_bind(pattern.clone());
        data_qb.push(clause).push_bind(pattern);
    }

     data_qb
        .push(" GROUP BY s.id, s.title, s.description, s.code, s.language, s.created_at")
        .push(" ORDER BY s.created_at DESC")
        .push(" LIMIT ").push_bind(per_page as i64)
        .push(" OFFSET ").push_bind(offset as i64);


    // --- Execute COUNT ---
    let (total_records,): (i64,) = count_qb
        .build_query_as()
        .fetch_one(&app_data.db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // --- Execute DATA fetch ---
    let records: Vec<SnippetData> = data_qb
        .build_query_as()
        .fetch_all(&app_data.db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let total_pages = ((total_records as f64) / (per_page as f64)).ceil() as u32;

    Ok(HttpResponse::Ok().json(PageResponse {
        total_records,
        total_pages,
        current_page,
        records,
    }))
}


#[derive(Deserialize)]
pub struct IdsParams {
    pub ids: String,
}
#[derive(Deserialize, Serialize, FromRow)]
pub struct SnippetCore {
    pub id:          Uuid,
    pub title:       String,
    pub description: Option<String>,
    pub code:        Option<String>,
    pub language:    String,
}

#[get("/batch")]
pub async fn get_snippets_by_ids(
    app_data: web::Data<AppState>,
    params:   web::Query<IdsParams>,
) -> actix_web::Result<impl Responder> {
    let ids: Vec<Uuid> = params
        .ids
        .split(',')
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();

    let records = sqlx::query_as!( 
        SnippetCore,
        r#"
        SELECT 
            id,
            title,
            description,
            code,
            language
        FROM snippets_extension.snippets 
        WHERE id = ANY($1)
        "#, 
        &ids[..] 
    )
    .fetch_all(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(records))
}


#[post("/{snippetId}/star")]
pub async fn star_snippet(
    app_data: web::Data<AppState>,
    path: web::Path<Uuid>,
    user_data: web::ReqData<UserData>,
) -> actix_web::Result<impl Responder> {
    let snippet_id = path.into_inner();
    let user_id = user_data.id;

    sqlx::query!(
        r#"
        INSERT INTO snippets_extension.snippet_stars (user_id, snippet_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
        user_id,
        snippet_id
    )
    .execute(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().finish())
}

#[delete("/{snippetId}/star")]
pub async fn unstar_snippet(
    app_data: web::Data<AppState>,
    path: web::Path<Uuid>,
    user_data: web::ReqData<UserData>,
) -> actix_web::Result<impl Responder> {
    let snippet_id = path.into_inner();
    let user_id = user_data.id;

    sqlx::query!(
        r#"
        DELETE FROM snippets_extension.snippet_stars
         WHERE user_id    = $1
           AND snippet_id = $2
        "#,
        user_id,
        snippet_id
    )
    .execute(&app_data.db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().finish())
}