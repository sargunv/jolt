module demo {
  exports a.api; // a api
  exports z.api; // z api

  uses a.Service; // a service
  uses z.Service; // z service
}
