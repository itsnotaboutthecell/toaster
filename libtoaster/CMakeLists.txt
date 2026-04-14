add_library(toaster SHARED
  toaster.c
  project.c
  analysis.c
  model.c
  callback/callback.c
)

target_compile_definitions(toaster PRIVATE toaster_EXPORTS)

target_include_directories(toaster
  PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}
)

if(WIN32)
  target_link_libraries(toaster PRIVATE shell32)
endif()
