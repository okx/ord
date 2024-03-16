use {
  super::{error::ApiError, *},
  axum::Json,
  utoipa::ToSchema,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiOutPointResult {
  pub result: Option<ApiSatRanges>,
  pub latest_blockhash: String,
  #[schema(format = "uint64")]
  pub latest_height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiSatRanges {
  /// The transaction id.
  pub outpoint: OutPoint,
  /// The script pubkey.
  pub sat_ranges: Vec<ApiSatRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum ApiSatRange {
  Sketchy((u64, u64)),
  #[serde(rename_all = "camelCase")]
  ExactWithRarity {
    first: u64,
    last: u64,
    rarity_sats: Vec<RaritySat>,
  },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RaritySat {
  pub sat: Sat,
  pub offset: u64,
  pub rarity: Rarity,
}

// /sat/outpoint/:outpoint/info
/// Retrieve the sat range of the outpoint.
#[utoipa::path(
    get,
    path = "/api/v1/sat/outpoint/{outpoint}/info",
    params(
        ("outpoint" = String, Path, description = "Outpoint")
  ),
    responses(
      (status = 200, description = "Obtain outpoint infomation", body = OrdOutPointData),
      (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
      (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
      (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
    )
  )]

pub(crate) async fn sat_range_by_outpoint(
  Extension(index): Extension<Arc<Index>>,
  Path(outpoint): Path<OutPoint>,
) -> ApiResult<ApiOutPointResult> {
  log::debug!("rpc: get sat_outpoint_sat_range: {outpoint}");

  let rtx = index.begin_read()?;

  let (latest_height, latest_blockhash) = rtx.latest_block()?.ok_or_api_err(|| {
    ApiError::internal("Failed to retrieve the latest block from the database.".to_string())
  })?;

  let sat_ranges = Index::list_sat_range(&rtx, outpoint, index.has_sat_index())?;

  Ok(Json(ApiResponse::ok(ApiOutPointResult {
    result: sat_ranges.map(|ranges| ApiSatRanges {
      outpoint,
      sat_ranges: ranges.into_iter().map(ApiSatRange::Sketchy).collect(),
    }),
    latest_height: latest_height.n(),
    latest_blockhash: latest_blockhash.to_string(),
  })))
}

pub(crate) async fn sat_range_with_rarity_by_outpoint(
  Extension(index): Extension<Arc<Index>>,
  Path(outpoint): Path<OutPoint>,
) -> ApiResult<ApiOutPointResult> {
  log::debug!("rpc: get sat_outpoint_sat_range: {outpoint}");

  let rtx = index.begin_read()?;

  let (latest_height, latest_blockhash) = rtx.latest_block()?.ok_or_api_err(|| {
    ApiError::internal("Failed to retrieve the latest block from the database.".to_string())
  })?;

  let Some(sat_ranges) = Index::list_sat_range(&rtx, outpoint, index.has_sat_index())? else {
    return Ok(Json(ApiResponse::ok(ApiOutPointResult {
      result: None,
      latest_height: latest_height.n(),
      latest_blockhash: latest_blockhash.to_string(),
    })));
  };

  let mut exact_sat_ranges = Vec::new();
  let mut value = 0;
  for sat_range in sat_ranges {
    let rarity_sats = Index::calculate_rarity_for_sat_range(sat_range)
      .into_iter()
      .map(|(sat, rarity)| RaritySat {
        sat,
        offset: sat.0 - sat_range.0 + value,
        rarity,
      })
      .collect();
    exact_sat_ranges.push(ApiSatRange::ExactWithRarity {
      first: sat_range.0,
      last: sat_range.1,
      rarity_sats,
    });
    value += sat_range.1 - sat_range.0;
  }

  Ok(Json(ApiResponse::ok(ApiOutPointResult {
    result: Some(ApiSatRanges {
      outpoint,
      sat_ranges: exact_sat_ranges,
    }),
    latest_height: latest_height.n(),
    latest_blockhash: latest_blockhash.to_string(),
  })))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_outpoint_sat_range_json_serialization() {
    let outpoint = unbound_outpoint();
    let sat_ranges = vec![(0, 100), (100, 200)];
    let api_outpoint_sat_ranges = ApiSatRanges {
      outpoint,
      sat_ranges: sat_ranges.into_iter().map(ApiSatRange::Sketchy).collect(),
    };
    let json = serde_json::to_string(&api_outpoint_sat_ranges).unwrap();
    assert_eq!(
      json,
      r#"{"outpoint":"0000000000000000000000000000000000000000000000000000000000000000:0","satRanges":[[0,100],[100,200]]}"#
    );
  }

  #[test]
  fn test_outpoint_sat_range_with_rarity_json_serialization() {
    let outpoint = unbound_outpoint();
    let rarity_sats = vec![
      RaritySat {
        sat: Sat(0),
        offset: 0,
        rarity: Rarity::Uncommon,
      },
      RaritySat {
        sat: Sat(1),
        offset: 1,
        rarity: Rarity::Epic,
      },
    ];
    let api_outpoint_sat_ranges = ApiSatRanges {
      outpoint,
      sat_ranges: vec![ApiSatRange::ExactWithRarity {
        first: 0,
        last: 100,
        rarity_sats,
      }],
    };
    let json = serde_json::to_string_pretty(&api_outpoint_sat_ranges).unwrap();
    assert_eq!(
      json,
      r##"{
  "outpoint": "0000000000000000000000000000000000000000000000000000000000000000:0",
  "satRanges": [
    {
      "first": 0,
      "last": 100,
      "raritySats": [
        {
          "sat": 0,
          "offset": 0,
          "rarity": "uncommon"
        },
        {
          "sat": 1,
          "offset": 1,
          "rarity": "epic"
        }
      ]
    }
  ]
}"##
    );
  }
}
