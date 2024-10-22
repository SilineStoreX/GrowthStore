<template>
  <div :class="className" :style="{height:height,width:width}" />
</template>

<script>
import echarts from 'echarts'
import * as macarons from 'echarts/theme/macarons'
import resize from './mixins/resize'
import * as datetime from '@/utils/datetime.js'

export default {
  mixins: [resize],
  props: {
    className: {
      type: String,
      default: 'chart'
    },
    width: {
      type: String,
      default: '100%'
    },
    height: {
      type: String,
      default: '250px'
    },
    autoResize: {
      type: Boolean,
      default: true
    },
    chartData: {
      type: Object,
      required: true
    }
  },
  data() {
    return {
      chart: null
    }
  },
  watch: {
    chartData: {
      deep: true,
      handler(val) {
        this.setOptions(val)
      }
    }
  },
  mounted() {
    this.$nextTick(() => {
      this.initChart()
    })
  },
  beforeDestroy() {
    if (!this.chart) {
      return
    }
    this.chart.dispose()
    this.chart = null
  },
  methods: {
    initChart() {
      this.chart = echarts.init(this.$el, 'macarons')
      this.setOptions(this.chartData)
    },
    setOptions({ data, title } = {}) {
      console.log(data)
      const xaxis = data.map(t => new Date(t.timestamp).strftime('%H:%M:%S'))
      const loaddata = data.map(t => (t.counter.task_2_count - (t.prev_counter ? t.prev_counter.task_2_count : 0)) / (t.diff_time))
      const errloaddata = data.map(t => (t.counter.task_3_count - (t.prev_counter ? t.prev_counter.task_3_count : 0)) / (t.diff_time))
      const postdata = data.map(t => (t.counter.task_4_count - (t.prev_counter ? t.prev_counter.task_4_count : 0)) / (t.diff_time))
      const errpostdata = data.map(t => (t.counter.task_5_count - (t.prev_counter ? t.prev_counter.task_5_count : 0)) / (t.diff_time))
      this.chart.setOption({
        title: {
          text: this.$t('任务处理情况'),
          x: 'left'
        },
        xAxis: {
          data: xaxis,
          boundaryGap: false,
          axisTick: {
            show: false
          }
        },
        grid: {
          left: 10,
          right: 10,
          bottom: 20,
          top: 40,
          containLabel: true
        },
        tooltip: {
          trigger: 'axis',
          axisPointer: {
            type: 'cross'
          },
          formatter: function(it) {
            return it.map(t => t.marker + t.seriesName + ': ' + t.value.toFixed(2) + '/S').join('<br/>')
          },
          padding: [5, 10]
        },
        yAxis: {
          axisTick: {
            show: false
          }
        },
        legend: {
          data: [this.$t('接收速率'), this.$t('接收出错量'), this.$t('处理速率'), this.$t('处理出错量')],
          left: 'right',
          top: '5px'
        },
        series: [{
          name: this.$t('接收速率'), itemStyle: {
            normal: {
              color: '#FF005A',
              lineStyle: {
                color: '#FF005A',
                width: 2
              },
              formatter: '{b}:({d}%)'
            }
          },
          smooth: true,
          type: 'line',
          data: loaddata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }, {
          name: this.$t('接收出错量'), itemStyle: {
            normal: {
              color: '#EC76FA',
              lineStyle: {
                color: '#EC76FA',
                width: 2
              },
              formatter: '{b}:({d}%)'
            }
          },
          smooth: true,
          type: 'line',
          data: errloaddata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }, {
          name: this.$t('处理速率'), itemStyle: {
            normal: {
              color: '#0F03DA',
              lineStyle: {
                color: '#0F03DA',
                width: 2
              },
              formatter: '{b}:({d}%)'
            }
          },
          smooth: true,
          type: 'line',
          data: postdata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }, {
          name: this.$t('处理出错量'), itemStyle: {
            normal: {
              color: '#CC4EAA',
              lineStyle: {
                color: '#CC4EAA',
                width: 2
              },
              formatter: '{b}:({d}%)'
            }
          },
          smooth: true,
          type: 'line',
          data: errpostdata,
          animationDuration: 2800,
          animationEasing: 'cubicInOut'
        }]
      })
    }
  }
}
</script>
