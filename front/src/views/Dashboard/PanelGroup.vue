<template>
  <el-row :gutter="40" class="panel-group">
    <el-col :xs="12" :sm="12" :lg="4" class="card-panel-col">
      <div class="card-panel" @click="handleSetLineChartData('member')">
        <div class="card-panel-icon-wrapper icon-date">
          <svg-icon icon-class="date" class-name="card-panel-icon" />
        </div>
        <div class="card-panel-description">
          <div class="card-panel-text">
            {{ this.$t('任务提交') }}
          </div>
          <span class="card-panel-num">{{ countData.counter.task_2_count }}</span>
        </div>
      </div>
    </el-col>    
    <el-col :xs="12" :sm="12" :lg="4" class="card-panel-col">
      <div class="card-panel" @click="handleSetLineChartData('member')">
        <div class="card-panel-icon-wrapper icon-date">
          <svg-icon icon-class="date" class-name="card-panel-icon" />
        </div>
        <div class="card-panel-description">
          <div class="card-panel-text">
            {{ this.$t('任务完成') }}
          </div>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="'错误数量：' + countData.counter.task_3_count + ' / 正常完成数量：' + countData.counter.task_1_count"
            placement="top-start"
          >
            <span class="card-panel-num">{{ countData.counter.task_1_count }}/<i class="red">{{ countData.counter.task_3_count }}</i></span>
          </el-tooltip>
        </div>
      </div>
    </el-col>
    <el-col :xs="12" :sm="12" :lg="4" class="card-panel-col">
      <div class="card-panel" @click="handleSetLineChartData('member')">
        <div class="card-panel-icon-wrapper icon-date">
          <svg-icon icon-class="date" class-name="card-panel-icon" />
        </div>
        <div class="card-panel-description">
          <div class="card-panel-text">
            {{ this.$t('长期任务') }} 
            <el-tooltip class="box-item" effect="dark" placement="top-start" content="活动的长期任务数（总数减已退出数）接近于工作线程数会引起系统性能下降。通常该值应该是恒定的。">
              <el-icon><InfoFilled /></el-icon>
            </el-tooltip>
          </div>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="'长期任务总数：' + countData.counter.task_4_count + ' / 已退出：' + countData.counter.task_5_count"
            placement="top-start"
          >
            <span class="card-panel-num">{{ countData.counter.task_4_count }}/<i class="red">{{ countData.counter.task_5_count }}</i></span>
          </el-tooltip>
        </div>
      </div>
    </el-col>

    <el-col :xs="12" :sm="12" :lg="4" class="card-panel-col">
      <div class="card-panel" @click="handleSetLineChartData('users')">
        <div class="card-panel-icon-wrapper icon-people">
          <svg-icon icon-class="peoples" class-name="card-panel-icon" />
        </div>
        <div class="card-panel-description">
          <div class="card-panel-text">
            {{ this.$t('总内存') }}
          </div>
          <span class="card-panel-num">{{ (countData.memory_total / 1024 / 1024 / 1024).toFixed(1) }} GB</span>
        </div>
      </div>
    </el-col>
    <el-col :xs="12" :sm="12" :lg="4" class="card-panel-col">
      <div class="card-panel" @click="handleSetLineChartData('questions')">
        <div class="card-panel-icon-wrapper icon-message">
          <svg-icon icon-class="message" class-name="card-panel-icon" />
        </div>
        <div class="card-panel-description">
          <div class="card-panel-text">
            {{ this.$t('CPU核心') }}
          </div>
          <span class="card-panel-num">{{ countData.cpu_cores }}</span>
        </div>
      </div>
    </el-col>
    <el-col :xs="12" :sm="12" :lg="4" class="card-panel-col">
      <div class="card-panel" @click="handleSetLineChartData('meeting')">
        <div class="card-panel-icon-wrapper icon-tree">
          <svg-icon icon-class="tree" class-name="card-panel-icon" />
        </div>
        <div class="card-panel-description">
          <div class="card-panel-text">
            {{ this.$t('线程数') }}
          </div>
          <el-tooltip
            class="box-item"
            effect="dark"
            :content="'线程总数：' + countData.threads + ' / 工作线程数：' + countData.counter.task_9_count + ' / 任务池化线程数：' + countData.counter.task_10_count"
            placement="top-start"
          >
            <span class="card-panel-num">{{ countData.threads }}/<i class="red">{{countData.counter.task_9_count}}</i></span>
          </el-tooltip>
        </div>
      </div>
    </el-col>
  </el-row>
</template>

<script>

export default {
  components: {},
  props: {
    countData: {
      type: Object,
      default: () => {}
    }
  },
  methods: {
    handleSetLineChartData(type) {
      this.$emit('handleSetLineChartData', type)
    }
  }
}
</script>

<style lang="scss" scoped>
.panel-group {
  margin-top: 18px;

  .card-panel-col {
    margin-bottom: 32px;
  }

  .card-panel {
    height: 108px;
    cursor: pointer;
    font-size: 12px;
    position: relative;
    overflow: hidden;
    color: #666;
    background: #fff;
    box-shadow: 4px 4px 40px rgba(0, 0, 0, .05);
    border-color: rgba(0, 0, 0, .05);
    display: flex;
    justify-content: flex-start;

    &:hover {
      .card-panel-icon-wrapper {
        color: #fff;
      }

      .icon-people {
        background: #40c9c6;
      }

      .icon-message {
        background: #36a3f7;
      }

      .icon-money {
        background: #f4516c;
      }

      .icon-date {
        background: #34bfa3
      }

      .icon-gonggao {
        background: #1ab9d4
      }

      .icon-tree {
        background: #02791f
      }
    }

    .icon-people {
      color: #40c9c6;
    }

    .icon-message {
      color: #36a3f7;
    }

    .icon-money {
      color: #f4516c;
    }

    .icon-date {
      color: #34bfa3
    }

    .icon-gonggao {
      color: #1ab9d4
    }

    .icon-tree {
      color: #02791f
    }

    .red {
      color: crimson;
    }

    .card-panel-icon-wrapper {
      margin: 14px 0 0 14px;
      padding: 16px;
      transition: all 0.38s ease-out;
      border-radius: 6px;
      height: 36px;
    }

    .card-panel-icon {
      float: left;
      font-size: 48px;
      width: 3em;
      height: 3em;
    }

    .card-panel-description {
      font-weight: bold;
      margin: 26px;
      display: flex;
      flex-direction: column;
      justify-content: center;
      align-items: center;

      .card-panel-text {
        line-height: 18px;
        color: rgba(0, 0, 0, 0.45);
        font-size: 16px;
        margin-bottom: 12px;
      }

      .card-panel-num {
        font-size: 20px;
      }
    }
  }
}

@media (max-width:550px) {
  .card-panel-description {
    display: none;
  }

  .card-panel-icon-wrapper {
    float: none !important;
    width: 100%;
    height: 100%;
    margin: 0 !important;

    .svg-icon {
      display: block;
      margin: 14px auto !important;
      float: none !important;
    }
  }
}
</style>
