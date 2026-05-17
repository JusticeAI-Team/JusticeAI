-- Repair records that were created from a non-UTF-8 console and contain
-- replacement question marks instead of readable Chinese text.

UPDATE labor_appeals
SET
    oral_description = CASE
        WHEN oral_description LIKE '%???%' OR length(oral_description) - length(replace(oral_description, '?', '')) >= 2
            THEN '申请人反映在北京市XX区XX地点附近务工期间被拖欠工资，具体金额、用工主体和证据材料仍需进一步核实。'
        ELSE oral_description
    END,
    wage_amount_text = CASE
        WHEN wage_amount_text LIKE '%???%' OR length(wage_amount_text) - length(replace(wage_amount_text, '?', '')) >= 2
            THEN '待核实欠薪金额'
        ELSE wage_amount_text
    END,
    employer_name = CASE
        WHEN employer_name LIKE '%???%' OR length(employer_name) - length(replace(employer_name, '?', '')) >= 2
            THEN '待核实用工主体'
        ELSE employer_name
    END,
    contractor_name = CASE
        WHEN contractor_name LIKE '%???%' OR length(contractor_name) - length(replace(contractor_name, '?', '')) >= 2
            THEN '待核实现场负责人'
        ELSE contractor_name
    END,
    project_name = CASE
        WHEN project_name LIKE '%???%' OR length(project_name) - length(replace(project_name, '?', '')) >= 2
            THEN '北京市XX区XX地点附近项目'
        ELSE project_name
    END,
    work_period_text = CASE
        WHEN work_period_text LIKE '%???%' OR length(work_period_text) - length(replace(work_period_text, '?', '')) >= 2
            THEN '待核实务工时间'
        ELSE work_period_text
    END,
    demand_text = CASE
        WHEN demand_text LIKE '%???%' OR length(demand_text) - length(replace(demand_text, '?', '')) >= 2
            THEN '希望协助核实欠薪情况并依法处理。'
        ELSE demand_text
    END,
    worker_name = CASE
        WHEN worker_name LIKE '%???%' OR length(worker_name) - length(replace(worker_name, '?', '')) >= 2
            THEN '张三'
        ELSE worker_name
    END,
    updated_at = NOW()
WHERE oral_description LIKE '%?%'
   OR wage_amount_text LIKE '%?%'
   OR employer_name LIKE '%?%'
   OR contractor_name LIKE '%?%'
   OR project_name LIKE '%?%'
   OR work_period_text LIKE '%?%'
   OR demand_text LIKE '%?%'
   OR worker_name LIKE '%?%';

UPDATE appeal_locations
SET
    area_name = CASE
        WHEN area_name LIKE '%???%' OR length(area_name) - length(replace(area_name, '?', '')) >= 2
            THEN '北京市XX区'
        ELSE area_name
    END,
    address_text = CASE
        WHEN address_text LIKE '%???%' OR length(address_text) - length(replace(address_text, '?', '')) >= 2
            THEN '北京市XX区XX地点附近'
        ELSE address_text
    END,
    conflict_flags = CASE
        WHEN conflict_flags LIKE '%???%' OR length(conflict_flags) - length(replace(conflict_flags, '?', '')) >= 2
            THEN ''
        ELSE conflict_flags
    END,
    updated_at = NOW()
WHERE area_name LIKE '%?%'
   OR address_text LIKE '%?%'
   OR conflict_flags LIKE '%?%';

UPDATE risk_cases rc
SET
    title = CASE
        WHEN rc.title LIKE '%???%' OR length(rc.title) - length(replace(rc.title, '?', '')) >= 2
            THEN COALESCE(NULLIF(la.project_name, ''), NULLIF(al.area_name, ''), '北京市XX区') || '欠薪诉求'
        ELSE rc.title
    END,
    area_name = CASE
        WHEN rc.area_name LIKE '%???%' OR length(rc.area_name) - length(replace(rc.area_name, '?', '')) >= 2
            THEN COALESCE(NULLIF(al.area_name, ''), '北京市XX区')
        ELSE rc.area_name
    END,
    risk_reason_summary = CASE
        WHEN rc.risk_reason_summary LIKE '%???%' OR length(rc.risk_reason_summary) - length(replace(rc.risk_reason_summary, '?', '')) >= 2
            THEN '由移动端欠薪诉求 ' || la.appeal_code || ' 转入。原始描述：' || la.oral_description
        ELSE rc.risk_reason_summary
    END,
    disposal_advice = CASE
        WHEN rc.disposal_advice LIKE '%???%' OR length(rc.disposal_advice) - length(replace(rc.disposal_advice, '?', '')) >= 2
            THEN '建议核实用工主体、欠薪金额、同项目类似线索并依法处置。'
        ELSE rc.disposal_advice
    END,
    risk_tags = CASE
        WHEN rc.risk_tags LIKE '%???%' OR length(rc.risk_tags) - length(replace(rc.risk_tags, '?', '')) >= 2
            THEN '欠薪,农民工,工程建设'
        ELSE rc.risk_tags
    END,
    updated_at = NOW()
FROM appeal_risk_case_links link
JOIN labor_appeals la ON la.id = link.appeal_id
LEFT JOIN appeal_locations al ON al.appeal_id = la.id
WHERE rc.id = link.risk_case_id
  AND (
      rc.title LIKE '%?%'
      OR rc.area_name LIKE '%?%'
      OR rc.risk_reason_summary LIKE '%?%'
      OR rc.disposal_advice LIKE '%?%'
      OR rc.risk_tags LIKE '%?%'
  );

UPDATE knowledge_entities ke
SET entity_name = CASE
    WHEN entity_type = 'case' THEN COALESCE(rc.title, '北京市XX区欠薪诉求')
    WHEN entity_type = 'project' THEN '北京市XX区XX地点附近项目'
    WHEN entity_type = 'organization' THEN '待核实用工主体'
    WHEN entity_type = 'person' THEN '待核实人员'
    WHEN entity_type = 'location' THEN COALESCE(rc.area_name, '北京市XX区')
    WHEN entity_type = 'risk_tag' THEN '欠薪'
    ELSE '待核实信息'
END
FROM risk_cases rc
WHERE ke.case_id = rc.id
  AND (ke.entity_name LIKE '%???%' OR length(ke.entity_name) - length(replace(ke.entity_name, '?', '')) >= 2);

UPDATE appeal_standardizations
SET
    standardized_title = '北京市XX区XX地点附近项目欠薪诉求',
    standard_summary = '申诉人反映在北京市XX区XX地点附近项目务工期间存在工资拖欠情况，具体欠薪金额、用工主体和证据链仍需人工核验。',
    standardized_text = '申诉人反映在北京市XX区XX地点附近项目务工期间存在工资拖欠情况，具体欠薪金额、用工主体和证据链仍需人工核验。建议补充工资记录、考勤记录、用工证明或聊天记录等材料。',
    extracted_fields = jsonb_set(
        jsonb_set(
            jsonb_set(COALESCE(extracted_fields, '{}'::jsonb), '{project_name}', '"北京市XX区XX地点附近项目"'::jsonb, true),
            '{area_name}', '"北京市XX区"'::jsonb, true
        ),
        '{wage_amount_text}', '"待核实欠薪金额"'::jsonb, true
    ),
    risk_case_mapping = jsonb_set(
        jsonb_set(COALESCE(risk_case_mapping, '{}'::jsonb), '{title}', '"北京市XX区XX地点附近项目欠薪诉求"'::jsonb, true),
        '{area_name}', '"北京市XX区"'::jsonb, true
    ),
    updated_at = NOW()
WHERE standardized_title LIKE '%?%'
   OR standard_summary LIKE '%?%'
   OR standardized_text LIKE '%?%';
